#[derive(Debug)]
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
    Array, // Only directly store scalars, containers must use traversals
    Object,
}

type Hierarchy = Vec<String>;

type ElementIdx = usize;

#[derive(Debug)]
pub struct Manager {
    elems: Vec<Element>,
}

impl Manager {
    pub fn new() -> Manager {
        Manager { elems: vec![] }
    }

    pub fn create_element(
        &mut self,
        name: &str,
        value: Value,
        parent: Option<ElementIdx>,
    ) -> ElementIdx {
        let idx = self.elems.len();
        self.elems.push(Element::new(name, value));

        if let Some(p_idx) = parent {
            self.elems[p_idx].children.push(idx);
            self.elems[idx].parent = parent;
        }

        idx
    }

    pub fn name(&self, idx: ElementIdx) -> &str {
        &self.elems[idx].name
    }

    pub fn value(&self, idx: ElementIdx) -> &Value {
        &self.elems[idx].value
    }

    pub fn lookup_element(&self, hierarchy: Hierarchy) -> Option<ElementIdx> {
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

    pub fn hierarchy_for(&self, e: ElementIdx) -> Hierarchy {
        let mut h: Hierarchy = vec![];
        let mut idx = e;

        loop {
            h.push(self.name(idx).to_owned());
            match self.elems[idx].parent {
                Some(p) => idx = p,
                None => break,
            }
        }

        h.reverse();
        h
    }
}

#[derive(Debug)]
struct Element {
    name: String,
    value: Value,
    parent: Option<ElementIdx>,
    children: Vec<ElementIdx>,
}

impl Element {
    fn new(name: &str, value: Value) -> Self {
        Element {
            name: name.to_owned(),
            value,
            parent: None,
            children: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_element() {
        let mut m = Manager::new();
        let e = m.create_element("alpha", Value::Str(String::from("Meowdy")), None);
        assert_eq!(m.name(e), "alpha");
    }

    #[test]
    fn create_child_get_hierarchy() {
        let mut m = Manager::new();
        let parent = m.create_element("parent", Value::I32(42), None);
        let child = m.create_element("child", Value::Bool(true), Some(parent));

        assert_eq!(m.hierarchy_for(child), vec!["parent", "child"]);
    }
}
