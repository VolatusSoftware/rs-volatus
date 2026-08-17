use volatus::config::{Manager, Value};

fn main() {
    let mut m = Manager::new();
    let p = m.create_element("alpha", Value::I32(42), None).unwrap();
    let c1 = m
        .create_element("beta", Value::Bool(true), Some(p))
        .unwrap();
    let _l = m
        .create_element("gamma", Value::Str("Meowdy".to_owned()), Some(c1))
        .unwrap();
    let _c2 = m
        .create_element("delta", Value::Bool(false), Some(p))
        .unwrap();

    println!("{m:?}");
}
