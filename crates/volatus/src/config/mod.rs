use std::fmt::Debug;

#[derive(Debug, PartialEq)]
pub enum Value {
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

#[derive(Debug)]
pub struct Manager {
    elems: Vec<Element>,
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            elems: vec![
                // Create root node as first element (idx == 0) to store names of top level elems
                // It's created as an object since it will hold named elements.
                Element::new("", Value::Object),
            ],
        }
    }

    pub fn create_element(
        &mut self,
        name: &str,
        value: Value,
        parent: Option<ElementIdx>,
    ) -> Result<ElementIdx, String> {
        if name.is_empty() || name == "Meta" || name == "Value" {
            return Err(format!("Element name '{name}' cannot be empty or the reserved strings 'Meta' or 'Value'."));
        }

        let idx = self.elems.len();
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

    pub fn name(&self, idx: ElementIdx) -> &str {
        &self.elems[idx].name
    }

    pub fn value(&self, idx: ElementIdx) -> &Value {
        &self.elems[idx].value
    }

    pub fn lookup_element(&self, hierarchy: Vec<&str>) -> Option<ElementIdx> {
        let mut idx: ElementIdx = 0;
        let mut child_found;
        for name in hierarchy {
            child_found = false;

            for child_idx in &self.elems[idx].children {
                if self.name(*child_idx) == name {
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

    pub fn parent_of(&self, idx: ElementIdx) -> Option<ElementIdx> {
        self.elems[idx].parent
    }

    pub fn hierarchy_for(&self, e: ElementIdx) -> Vec<String> {
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

//#[derive(Debug)]
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
        assert!(m.create_element("", Value::None, None).is_err());
    }

    #[test]
    fn reserved_element_name_fails() {
        let mut m = Manager::new();
        assert!(m.create_element("Meta", Value::None, None).is_err());
        assert!(m.create_element("Value", Value::None, None).is_err());
    }

    #[test]
    fn create_element() {
        let mut m = Manager::new();
        let e = m.create_element("alpha", Value::Str(String::from("Meowdy")), None).unwrap();
        assert_eq!(m.name(e), "alpha");
    }

    #[test]
    fn create_child_get_hierarchy() {
        let mut m = Manager::new();
        let parent = m.create_element("parent", Value::I32(42), None).unwrap();
        let child = m.create_element("child", Value::Bool(true), Some(parent)).unwrap();

        assert_eq!(m.hierarchy_for(child), vec!["parent", "child"]);
    }

    #[test]
    fn traverse_to_parent() {
        let mut m = Manager::new();
        let p = m.create_element("parent", Value::None, None).unwrap();
        let c = m.create_element("child", Value::None, Some(p)).unwrap();

        let tp = m.parent_of(c).unwrap();
        assert_eq!(m.name(tp), "parent");
    }

    #[test]
    fn lookup_hierarchy() {
        let mut m = Manager::new();
        let p = m.create_element("parent", Value::None, None).unwrap();
        let _c = m.create_element("child", Value::I32(42), Some(p)).unwrap();

        let e = m.lookup_element(vec!["parent", "child"]).unwrap();
        assert_eq!(*m.value(e), Value::I32(42));
    }
}
