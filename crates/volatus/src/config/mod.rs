use std::{collections::VecDeque, fmt::Debug};

#[derive(Debug, PartialEq)]
pub enum Value {
    Vacant,
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

type ElementIdx = usize;

fn validate_elem_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "Meta" || name == "Value" {
        return Err(format!(
            "Element name '{name}' cannot be empty or the reserved strings 'Meta' or 'Value'."
        ));
    }

    Ok(())
}

#[derive(Debug)]
pub struct Manager {
    elems: Vec<Element>,
    vacant: Vec<ElementIdx>,
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            elems: vec![
                // Create root node as first element (idx == 0) to store names of top level elems
                // It's created as an object since it will hold named elements.
                Element::new("", Value::Object),
            ],
            vacant: vec![],
        }
    }

    pub fn create(
        &mut self,
        name: &str,
        value: Value,
        parent: Option<ElementIdx>,
    ) -> Result<ElementIdx, String> {
        validate_elem_name(name)?;

        let idx = if self.vacant.len() > 0 {
            self.vacant.pop().unwrap()
        } else {
            self.elems.len()
        };
        self.elems.push(Element::new(name, value));

        // New elements are always linked to from a parent
        // If new element parent was not specified then use the root element
        //  so that "top-level" names can be looked up.
        let p_idx = parent.unwrap_or_else(|| 0);
        self.elems[p_idx].children.push(idx);

        // New elements do not link back to the root node
        // This simplifies hierarchy lookups and generation
        if p_idx > 0 {
            self.elems[idx].parent = Some(p_idx);
        };

        Ok(idx)
    }

    pub fn remove(&mut self, idx: ElementIdx) -> Result<(), String> {
        let mut to_remove = VecDeque::<ElementIdx>::from([idx]);

        let p = self.parent(idx).unwrap_or_else(|| 0);
        self.elems[p].children.retain(|&x| x != idx);

        while !to_remove.is_empty() {
            let idx = to_remove.pop_front().unwrap();
            for child_idx in &self.elems[idx].children {
                to_remove.push_back(*child_idx);
            }

            // To make sure existing indices aren't invalidated and don't have
            //  to be adjusted, a "blank" is replaced into the vec.
            // The position is tracked in vacant so the now "blank" position can
            //  be used for new elements.
            self.vacant.push(idx);
            self.elems[idx].value = Value::Vacant;
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.elems.len() - 1 - self.vacant.len()
    }

    pub fn name(&self, idx: ElementIdx) -> &str {
        &self.elems[idx].name
    }

    pub fn value(&self, idx: ElementIdx) -> &Value {
        &self.elems[idx].value
    }

    pub fn value_mut(&mut self, idx: ElementIdx) -> &mut Value {
        &mut self.elems[idx].value
    }

    pub fn rename(&mut self, idx: ElementIdx, name: &str) -> Result<(), String> {
        validate_elem_name(name)?;

        self.elems[idx].name = name.to_owned();

        Ok(())
    }

    pub fn obtain(&mut self, hierarchy: &Vec<&str>) -> Result<ElementIdx, String> {
        let mut child_found;
        let mut idx = 0;

        for name in hierarchy {
            child_found = false;

            for child_idx in &self.elems[idx].children {
                if self.name(*child_idx) == *name {
                    idx = *child_idx;
                    child_found = true;
                    break;
                }
            }

            if !child_found {
                let e =
                    self.create(*name, Value::None, if idx > 0 { Some(idx) } else { None });
                match e {
                    Ok(new_idx) => idx = new_idx,
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(idx)
    }

    pub fn lookup(&self, hierarchy: &Vec<&str>) -> Option<ElementIdx> {
        let mut idx: ElementIdx = 0;
        let mut child_found;
        for name in hierarchy {
            child_found = false;

            for child_idx in &self.elems[idx].children {
                if self.name(*child_idx) == *name {
                    idx = *child_idx;
                    child_found = true;
                    break;
                }
            }

            if !child_found {
                return None;
            }
        }

        Some(idx)
    }

    pub fn parent(&self, idx: ElementIdx) -> Option<ElementIdx> {
        self.elems[idx].parent
    }

    pub fn is_vacant(&self, idx: ElementIdx) -> bool {
        self.elems[idx].value == Value::Vacant
    }

    pub fn hierarchy(&self, e: ElementIdx) -> Vec<String> {
        let mut h = vec![];
        let mut idx = e;

        // Follow parent index until no further parent links.
        loop {
            h.push(self.name(idx).to_owned());
            match self.elems[idx].parent {
                Some(p) => idx = p,
                None => break,
            }
        }

        // Since we worked from child-most towards root, need to fix back from parent-most to element.
        h.reverse();

        h
    }
}

struct Element {
    name: String,
    value: Value,
    parent: Option<ElementIdx>,
    children: Vec<ElementIdx>,
}

impl Element {
    fn new(name: &str, value: Value) -> Self {
        // New elements start out orphaned and Manager will link appropriately.
        Element {
            name: name.to_owned(),
            value,
            parent: None,
            children: vec![],
        }
    }
}

impl Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("");
        d.field("name", &self.name);
        d.field("value", &self.value);

        if let Some(p) = self.parent {
            d.field("parent", &p);
        }

        if self.children.len() > 0 {
            d.field("children", &self.children);
        }

        d.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_element_name_fails() {
        let mut m = Manager::new();
        assert!(m.create("", Value::None, None).is_err());
    }

    #[test]
    fn reserved_element_name_fails() {
        let mut m = Manager::new();
        assert!(m.create("Meta", Value::None, None).is_err());
        assert!(m.create("Value", Value::None, None).is_err());
    }

    #[test]
    fn reserved_rename_fails() {
        let mut m = Manager::new();
        let e = m.create("alpha", Value::None, None).unwrap();
        assert!(m.rename(e, "Meta").is_err());
    }

    #[test]
    fn rename_element() {
        let mut m = Manager::new();
        let e = m.create("alpha", Value::None, None).unwrap();
        m.rename(e, "beta").unwrap();
        assert_eq!(m.name(e), "beta");
    }

    #[test]
    fn create_element() {
        let mut m = Manager::new();
        let e = m
            .create("alpha", Value::Str(String::from("Meowdy")), None)
            .unwrap();
        assert_eq!(m.name(e), "alpha");
    }

    #[test]
    fn create_child_get_hierarchy() {
        let mut m = Manager::new();
        let parent = m.create("parent", Value::I32(42), None).unwrap();
        let child = m
            .create("child", Value::Bool(true), Some(parent))
            .unwrap();

        assert_eq!(m.hierarchy(child), vec!["parent", "child"]);
    }

    #[test]
    fn traverse_to_parent() {
        let mut m = Manager::new();
        let p = m.create("parent", Value::None, None).unwrap();
        let c = m.create("child", Value::None, Some(p)).unwrap();

        let tp = m.parent(c).unwrap();
        assert_eq!(m.name(tp), "parent");
    }

    #[test]
    fn lookup_hierarchy() {
        let mut m = Manager::new();
        let p = m.create("parent", Value::None, None).unwrap();
        let _c = m.create("child", Value::I32(42), Some(p)).unwrap();

        let e = m.lookup(&vec!["parent", "child"]).unwrap();
        assert_eq!(*m.value(e), Value::I32(42));
    }

    #[test]
    fn change_value() {
        let mut m = Manager::new();
        let e = m.create("alpha", Value::None, None).unwrap();
        *m.value_mut(e) = Value::Bool(true);
        assert_eq!(*m.value(e), Value::Bool(true));
    }

    #[test]
    fn create_via_obtain() {
        let mut m = Manager::new();
        let _ = m.create("alpha", Value::None, None).unwrap();
        let e = m.obtain(&vec!["alpha", "beta", "gamma"]).unwrap();
        assert_eq!(m.hierarchy(e), vec!["alpha", "beta", "gamma"]);
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn remove_subtree() {
        let mut m = Manager::new();
        let a = m.create("a", Value::None, None).unwrap();
        let b = m.create("b", Value::None, Some(a)).unwrap();
        let c = m.create("c", Value::None, Some(a)).unwrap();
        let d = m.create("d", Value::None, Some(c)).unwrap();

        m.remove(c).unwrap();
        
        assert_eq!(m.len(), 2);
        assert_eq!(m.hierarchy(b), vec!["a", "b"]);
        assert!(m.is_vacant(d));
    }
}
