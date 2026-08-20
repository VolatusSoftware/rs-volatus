use std::{
    collections::{BTreeMap, VecDeque}, fmt::Debug,
};

use serde_json::{Value, Map};

#[derive(Debug, PartialEq)]
pub enum ElemVal {
    Vacant, // Filler when an element has been "removed" but don't want to shuffle indices
    None,
    Str(String),
    Bool(bool),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    Array, // Only directly store scalars, containers use traversals
    Object,
}

fn validate_elem_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "Meta" || name == "Value" {
        return Err(format!(
            "Element name '{name}' cannot be empty or the reserved strings 'Meta' or 'Value'."
        ));
    }

    Ok(())
}

#[derive(Debug)]
enum Lookup<'a> {
    Name(&'a str),
    HasMeta(&'a str),
    Meta { name: &'a str, value: &'a str },
}

#[derive(Debug)]
pub struct ElemLookup<'a> {
    lookups: Vec<Lookup<'a>>,
}

impl<'a> ElemLookup<'a> {
    pub fn new() -> Self {
        ElemLookup { lookups: vec![] }
    }

    pub fn match_name(mut self, name: &'a str) -> Self {
        self.lookups.push(Lookup::Name(name));
        self
    }

    pub fn match_has_meta(mut self, name: &'a str) -> Self {
        self.lookups.push(Lookup::HasMeta(name));
        self
    }

    pub fn match_meta(mut self, name: &'a str, value: &'a str) -> Self {
        self.lookups.push(Lookup::Meta { name, value });
        self
    }

    fn matches(&self, e: &Element) -> bool {
        for lookup in &self.lookups {
            match *lookup {
                Lookup::Name(name) => {
                    if name != e.name {
                        return false;
                    }
                }
                Lookup::HasMeta(name) => {
                    if !e.meta.contains_key(name) {
                        return false;
                    }
                }
                Lookup::Meta { name, value } => {
                    if let Some(v) = e.meta.get(name) {
                        if v != value {
                            return false;
                        }
                    }
                }
            }
        }

        true // matches unless mismatch found
    }
}

#[derive(Clone, Copy)]
pub struct ElemHandle(usize);

impl ElemHandle {
    fn new(i: usize) -> Self {
        ElemHandle(i)
    }

    fn root() -> Self {
        ElemHandle(0)
    }

    pub fn new_child(
        self,
        mgr: &mut Manager,
        name: &str,
        value: ElemVal,
    ) -> Result<ElemHandle, String> {
        mgr.create(name, value, Some(self))
    }

    fn is_root(&self) -> bool {
        self.0 == 0
    }
}

impl Debug for ElemHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

struct Element {
    name: String,
    value: ElemVal,
    parent: Option<ElemHandle>,
    children: Vec<ElemHandle>,
    meta: BTreeMap<String, String>,
}

impl Element {
    fn new(name: &str, value: ElemVal) -> Self {
        // New elements start out orphaned and Manager will link appropriately.
        Element {
            name: name.to_owned(),
            value,
            parent: None,
            children: vec![],
            meta: BTreeMap::new(),
        }
    }
}

impl Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value {
            ElemVal::Vacant => write!(f, "VACANT"),
            _ => {
                let mut d = f.debug_struct("");

                if !self.name.is_empty() {
                    d.field("name", &self.name);
                    d.field("value", &self.value);

                    if let Some(p) = &self.parent {
                        d.field("parent", &p.0);
                    }
                }

                if self.children.len() > 0 {
                    d.field("children", &self.children);
                }

                d.finish()
            }
        }
    }
}

pub struct JsonLoader {}

struct JsonElem {
    e: ElemHandle,
    arr_val: Option<Vec<Value>>,
    obj_val: Option<Map<String, Value>>,
}

impl JsonLoader {
    fn add_elem(
        frontier: &mut VecDeque<JsonElem>,
        m: &mut Manager,
        parent: ElemHandle,
        val: Value,
        name: String,
    ) -> Result<(), String> {
        if name == "Meta" {
            match val {
                Value::Object(o) => {
                    for (meta_name, meta_val) in o {
                        if meta_val.is_string() {
                            m.set_meta(parent, &meta_name, meta_val.as_str().unwrap());
                        } else {
                            return Err(format!("Meta value '{meta_name}' under '{}' must have a string value.", m.name(parent)));
                        }
                    }
                }
                _ => {
                    return Err(format!("Meta field under '{}' must be an object.", m.name(parent)));
                }
            }
        } else {
            let mut arr_val: Option<Vec<Value>> = None;
            let mut obj_val: Option<Map<String, Value>> = None;

            let ev = match val {
                Value::Null => ElemVal::None,
                Value::Bool(b) => ElemVal::Bool(b),
                Value::Number(n) => {
                    if n.is_f64() {
                        ElemVal::F64(n.as_f64().unwrap())
                    } else if n.is_i64() {
                        ElemVal::I64(n.as_i64().unwrap())
                    } else {
                        ElemVal::U64(n.as_u64().unwrap())
                    }
                }
                Value::String(s) => ElemVal::Str(s),
                Value::Array(a) => {
                    arr_val = Some(a);
                    ElemVal::Array
                }
                Value::Object(o) => {
                    obj_val = Some(o);
                    ElemVal::Object
                }
            };
            
            let e = parent.new_child(m, &name, ev)?;

            frontier.push_back(JsonElem {
                e,
                arr_val,
                obj_val,
            });
        }

        Ok(())
    }

    pub fn from_json(json: &str) -> Result<Manager, String> {
        let v: serde_json::Value = match serde_json::from_str(json) {
            Ok(value) => value,
            Err(e) => {
                return Err(format!("{e}"));
            }
        };

        let obj = if let Value::Object(o) = v { o } else {
            return Err("JSON value must be an object.".to_owned());
        };

        let mut m = Manager::new();

        let mut frontier: VecDeque<JsonElem> = VecDeque::new();
        //Self::add_elem(&mut frontier, &mut m, root, v, "".to_owned())?;
        frontier.push_back(JsonElem {
            e: m.root(),
            arr_val: None,
            obj_val: Some(obj),
        });

        while !frontier.is_empty() {
            let elem = frontier.pop_front().unwrap();
            match elem {
                JsonElem { arr_val: Some(a), .. } => {
                    let mut i = 0;
                    for arr_elem in a {
                        Self::add_elem(&mut frontier, &mut m, elem.e, arr_elem, format!("{i}"))?;
                        i += 1;
                    }
                }
                JsonElem { obj_val: Some(o), .. } => {
                    for (obj_name, obj_elem) in o {
                        Self::add_elem(&mut frontier, &mut m, elem.e, obj_elem, obj_name)?;
                    }
                }
                _ => {}
            };
        };

        Ok(m)
    }
}

#[derive(Debug)]
pub struct Manager {
    elems: Vec<Element>,
    vacant: Vec<usize>, // Tracks available vacant positions in the elems vector
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            elems: vec![
                // Create root node as first element (idx == 0) to store names of top level elems
                // It's created as an object since it will hold named elements.
                Element::new("", ElemVal::Object),
            ],
            vacant: vec![],
        }
    }

    pub fn root(&self) -> ElemHandle {
        ElemHandle(0)
    }

    /// Creates a new element in the tree, optional as a child of an existing parent.
    pub fn create(
        &mut self,
        name: &str,
        value: ElemVal,
        parent: Option<ElemHandle>,
    ) -> Result<ElemHandle, String> {
        validate_elem_name(name)?;

        // ElemHandle will either be a vacant spot from a deletion or the end of the vec
        let e = ElemHandle::new(if self.vacant.len() > 0 {
            self.vacant.pop().unwrap()
        } else {
            self.elems.len()
        });

        self.elems.push(Element::new(name, value));

        // New elements are always linked to from a parent
        // If new element parent was not specified then use the root element
        //  so that "top-level" names can be looked up.
        let p = parent.unwrap_or_else(|| ElemHandle::root());
        self.elems[p.0].children.push(e);

        // New elements do not link back to the root node
        // This simplifies hierarchy lookups and generation
        if !p.is_root() {
            self.elems[e.0].parent = Some(p);
        };

        Ok(e)
    }

    fn elem(&self, e: ElemHandle) -> &Element {
        &self.elems[e.0]
    }

    fn elem_mut(&mut self, e: ElemHandle) -> &mut Element {
        &mut self.elems[e.0]
    }

    fn meta(&self, e: ElemHandle) -> &BTreeMap<String, String> {
        &self.elem(e).meta
    }

    fn meta_mut(&mut self, e: ElemHandle) -> &mut BTreeMap<String, String> {
        &mut self.elem_mut(e).meta
    }

    pub fn set_meta(&mut self, e: ElemHandle, name: &str, value: &str) {
        self.meta_mut(e).insert(name.to_owned(), value.to_owned());
    }

    pub fn get_meta(&self, e: ElemHandle, name: &str) -> Option<&String> {
        self.meta(e).get(name)
    }

    pub fn has_meta(&self, e: ElemHandle, name: &str) -> bool {
        self.meta(e).contains_key(name)
    }

    /// Removes an element from the tree along with all of its descendents.
    pub fn remove(&mut self, e: ElemHandle) -> Result<(), String> {
        let mut to_remove = VecDeque::<ElemHandle>::from([e]);

        let p = self.parent(e).unwrap_or_else(|| ElemHandle::root());
        self.children_mut(p).retain(|&x| x.0 != e.0);

        while !to_remove.is_empty() {
            let e = to_remove.pop_front().unwrap();
            for c in self.children(e) {
                to_remove.push_back(*c);
            }

            // To make sure existing indices aren't invalidated and don't have
            //  to be adjusted, a "blank" is replaced into the vec.
            // The position is tracked in vacant so the now "blank" position can
            //  be used for new elements.
            self.vacant.push(e.0);
            self.elem_mut(e).value = ElemVal::Vacant;
        }

        Ok(())
    }

    /// Returns the number of active elements in the tree. This does not include the
    /// hidden root element.
    pub fn len(&self) -> usize {
        self.elems.len() - 1 - self.vacant.len()
    }

    /// Returns the name of an element given its handle.
    pub fn name(&self, e: ElemHandle) -> &str {
        &self.elem(e).name
    }

    /// Returns an immutable reference of an element's value given its handle.
    /// Use `value_mut()` for a mutable reference.
    pub fn value(&self, e: ElemHandle) -> &ElemVal {
        &self.elem(e).value
    }

    /// Returns a mutable reference to an element's value so it can be changed.
    pub fn value_mut(&mut self, e: ElemHandle) -> &mut ElemVal {
        &mut self.elem_mut(e).value
    }

    /// Changes the name for an element given its index and the new name.
    pub fn rename(&mut self, e: ElemHandle, name: &str) -> Result<(), String> {
        validate_elem_name(name)?;

        self.elem_mut(e).name = name.to_owned();

        Ok(())
    }

    pub fn children(&self, e: ElemHandle) -> &Vec<ElemHandle> {
        &self.elem(e).children
    }

    pub fn children_mut(&mut self, e: ElemHandle) -> &mut Vec<ElemHandle> {
        &mut self.elem_mut(e).children
    }

    pub fn descendents(
        &self,
        e: ElemHandle,
        lookup: &ElemLookup,
        look_past_match: bool,
    ) -> Vec<ElemHandle> {
        let mut q = VecDeque::new();
        let mut found = Vec::new();

        //preload with direct children to begin check
        for c in self.children(e) {
            q.push_back(*c);
        }

        while !q.is_empty() {
            let e = q.pop_front().unwrap();
            let matches = lookup.matches(self.elem(e));

            if matches {
                found.push(e)
            };

            if !matches || look_past_match {
                for c in self.children(e) {
                    q.push_back(*c);
                }
            }
        }

        found
    }

    pub fn ancestor(&self, e: ElemHandle, lookup: &ElemLookup) -> Option<ElemHandle> {
        let mut e = self.elem(e);
        loop {
            match e.parent {
                Some(ph) => {
                    if lookup.matches(self.elem(ph)) {
                        return Some(ph);
                    } else {
                        e = self.elem(ph)
                    }
                }
                None => return None,
            }
        }
    }

    /// Retrieves an element at the specified hierarchy.
    /// Any ancestor elements in the hierarchy that do not exist will be created
    ///  along with the element itself.
    pub fn obtain(&mut self, hierarchy: &Vec<&str>) -> Result<ElemHandle, String> {
        let mut child_found;
        let mut e = ElemHandle::root();

        for name in hierarchy {
            child_found = false;

            for c in self.children(e) {
                if self.name(*c) == *name {
                    e = *c;
                    child_found = true;
                    break;
                }
            }

            if !child_found {
                let result = self.create(
                    *name,
                    ElemVal::None,
                    if !e.is_root() { Some(e) } else { None },
                );
                e = match result {
                    Ok(h) => h,
                    Err(e) => return Err(e),
                };
            }
        }

        Ok(e)
    }

    /// Retrieves an element handle given its named hierarchy. If the element does not
    /// exist, `None` will be returned.
    pub fn lookup(&self, hierarchy: &Vec<&str>) -> Option<ElemHandle> {
        let mut e = ElemHandle::root();
        let mut child_found;
        for name in hierarchy {
            child_found = false;

            for c in self.children(e) {
                if self.name(*c) == *name {
                    e = *c;
                    child_found = true;
                    break;
                }
            }

            if !child_found {
                return None;
            }
        }

        Some(e)
    }

    pub fn lookup_child(&self, e: ElemHandle, name: &str) -> Option<ElemHandle> {
        for c in self.children(e) {
            if self.name(*c) == name {
                return Some(*c);
            }
        }

        None
    }

    /// Returns the parent handle of the specified element, or `None` if the element
    /// does not have a parent.
    pub fn parent(&self, e: ElemHandle) -> Option<ElemHandle> {
        self.elem(e).parent
    }

    /// Returns whether or not the element handle is a vacant slot.
    /// This means the handle is not valid for other operations.
    pub fn is_vacant(&self, e: ElemHandle) -> bool {
        self.elem(e).value == ElemVal::Vacant
    }

    /// Returns the name hierarchy of the specified element index.
    pub fn hierarchy(&self, e: ElemHandle) -> Vec<String> {
        let mut h = vec![];
        let mut e = e;

        // Follow parent index until no further parent links.
        loop {
            h.push(self.name(e).to_owned());
            match self.parent(e) {
                Some(p) => e = p,
                None => break,
            }
        }

        // Since we worked from child-most towards root, need to reverse from parent-most to element.
        h.reverse();

        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_element_name_fails() {
        let mut m = Manager::new();
        assert!(m.create("", ElemVal::None, None).is_err());
    }

    #[test]
    fn reserved_element_name_fails() {
        let mut m = Manager::new();
        assert!(m.create("Meta", ElemVal::None, None).is_err());
        assert!(m.create("Value", ElemVal::None, None).is_err());
    }

    #[test]
    fn reserved_rename_fails() {
        let mut m = Manager::new();
        let e = m.create("alpha", ElemVal::None, None).unwrap();
        assert!(m.rename(e, "Meta").is_err());
    }

    #[test]
    fn rename_element() {
        let mut m = Manager::new();
        let e = m.create("alpha", ElemVal::None, None).unwrap();
        m.rename(e, "beta").unwrap();
        assert_eq!(m.name(e), "beta");
    }

    #[test]
    fn create_element() {
        let mut m = Manager::new();
        let e = m
            .create("alpha", ElemVal::Str(String::from("Meowdy")), None)
            .unwrap();
        assert_eq!(m.name(e), "alpha");
    }

    #[test]
    fn create_child_get_hierarchy() {
        let mut m = Manager::new();
        let parent = m.create("parent", ElemVal::I32(42), None).unwrap();
        let child = m
            .create("child", ElemVal::Bool(true), Some(parent))
            .unwrap();

        assert_eq!(m.hierarchy(child), vec!["parent", "child"]);
    }

    #[test]
    fn traverse_to_parent() {
        let mut m = Manager::new();
        let p = m.create("parent", ElemVal::None, None).unwrap();
        let c = m.create("child", ElemVal::None, Some(p)).unwrap();

        let tp = m.parent(c).unwrap();
        assert_eq!(m.name(tp), "parent");
    }

    #[test]
    fn lookup_hierarchy() {
        let mut m = Manager::new();
        let p = m.create("parent", ElemVal::None, None).unwrap();
        let _c = m.create("child", ElemVal::I32(42), Some(p)).unwrap();

        let e = m.lookup(&vec!["parent", "child"]).unwrap();
        assert_eq!(*m.value(e), ElemVal::I32(42));
    }

    #[test]
    fn change_value() {
        let mut m = Manager::new();
        let e = m.create("alpha", ElemVal::None, None).unwrap();
        *m.value_mut(e) = ElemVal::Bool(true);
        assert_eq!(*m.value(e), ElemVal::Bool(true));
    }

    #[test]
    fn create_via_obtain() {
        let mut m = Manager::new();
        let _ = m.create("alpha", ElemVal::None, None).unwrap();
        let e = m.obtain(&vec!["alpha", "beta", "gamma"]).unwrap();
        assert_eq!(m.hierarchy(e), vec!["alpha", "beta", "gamma"]);
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn remove_subtree() {
        let mut m = Manager::new();
        let a = m.create("a", ElemVal::None, None).unwrap();
        let b = m.create("b", ElemVal::None, Some(a)).unwrap();
        let c = m.create("c", ElemVal::None, Some(a)).unwrap();
        let d = m.create("d", ElemVal::None, Some(c)).unwrap();

        m.remove(c).unwrap();

        assert_eq!(m.len(), 2);
        assert_eq!(m.hierarchy(b), vec!["a", "b"]);
        assert!(m.is_vacant(d));
    }

    #[test]
    fn set_get_meta() {
        let mut m = Manager::new();
        let e = m.create("a", ElemVal::None, None).unwrap();
        m.set_meta(e, "VL_Type", "VL_Task");

        assert!(m.has_meta(e, "VL_Type"));
        assert_eq!(m.get_meta(e, "VL_Type").unwrap(), "VL_Task");
    }

    #[test]
    fn get_missing_meta() {
        let mut m = Manager::new();
        let e = m.create("a", ElemVal::None, None).unwrap();

        assert!(!m.has_meta(e, "VL_Type"));
        assert!(m.get_meta(e, "VL_Type").is_none());
    }

    #[test]
    fn match_meta_name() {
        let mut m = Manager::new();
        let e = m.create("a", ElemVal::None, None).unwrap();
        let l = ElemLookup::new().match_name("a");

        assert!(l.matches(m.elem(e)));
    }

    #[test]
    fn descendent_meta() {
        let mut m = Manager::new();
        let a = m.create("a", ElemVal::None, None).unwrap();
        let b = a.new_child(&mut m, "b", ElemVal::None).unwrap();
        let c = b.new_child(&mut m, "c", ElemVal::None).unwrap();
        let d = c.new_child(&mut m, "d", ElemVal::None).unwrap();

        m.set_meta(a, "VL_Type", "VL_Task");
        m.set_meta(b, "VL_Type", "VL_Channel");
        m.set_meta(c, "VL_Type", "VL_Group");
        m.set_meta(d, "VL_Type", "VL_Channel");

        let l = ElemLookup::new().match_meta("VL_Type", "VL_Channel");

        let chans = m.descendents(a, &l, true);

        assert_eq!(chans.len(), 2);

        let names: Vec<&str> = chans.iter().map(|e| m.name(*e)).collect();
        assert!(names.contains(&m.name(b)));
        assert!(names.contains(&m.name(d)));
    }

    #[test]
    fn basic_parse_json() {
        let json = r#"
{
    "Volatus": {
        "Meta": {"VL_Config_Version": "0.0.0-dev"},
        "TestSystem": {
            "Meta": {"VL_Type": "VL_System"},
            "Clusters": {
                "TestCluster": {
                    "Meta": {"VL_Type": "VL_Cluster"},
                    "Telemetry": {"Routing": "Unicast", "Endpoint": {"Address": "127.0.0.1", "Port": 36985} },
                    "Groups": {"Events": 5, "Config": 6, "Logging": 7, "Clients": 8},
                    "Nodes": {
                        "Test_DAQ": {
                            "Meta": {"VL_Type": "VL_Node"},
                            "Node_ID": 1,
                            "Network": {
                                "Bind_Address": "0.0.0.0",
                                "TCP": {"Address": "127.0.0.1", "Port": 12081, "Server": true},
                                "HTTP_Port": 13081
                            },
                            "Tasks": {
                                "Telem_Server": {"Meta": {"VL_Type": "VL_Task", "VL_Task_Type": "TelemetryServer"}}
                            }
                        },
                        "Test_GUI": {
                            "Meta": {"VL_Type": "VL_Node"},
                            "Node_ID": 2,
                            "Network": {
                                "Bind_Address": "0.0.0.0",
                                "TCP": {"Address": "127.0.0.1", "Port": 12081, "Server": false},
                                "HTTP_Port": 13081
                            },
                            "Tasks": {
                                "GUI_Manager": {
                                    "Meta": {"VL_Type": "VL_Task", "VL_Task_Type": "GUIManager"},
                                    "Plugins": "gui",
                                    "Title": "Volatus Test"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}"#;

        let m = JsonLoader::from_json(json).unwrap();
        println!("{m:#?}");
    }
}
