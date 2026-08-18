use volatus::config::{Manager, Value};

fn main() {
    let mut m = Manager::new();

    let p = m.create("alpha", Value::I32(42), None).unwrap();

    let c1 = p.new_child(&mut m, "beta", Value::Bool(true)).unwrap();
    
    let l = c1
        .new_child(&mut m, "gamma", Value::Str("Meowdy".to_owned()))
        .unwrap();
    
    let _c2 = p.new_child(&mut m, "delta", Value::Bool(false)).unwrap();

    m.remove(l).unwrap();

    println!("{m:?}");
}
