use volatus::config::{Manager, ElemVal};

fn main() {
    let mut m = Manager::new();

    let p = m.create("alpha", ElemVal::I32(42), None).unwrap();

    let c1 = p.new_child(&mut m, "beta", ElemVal::Bool(true)).unwrap();
    
    let l = c1
        .new_child(&mut m, "gamma", ElemVal::Str("Meowdy".to_owned()))
        .unwrap();

    let _c2 = p.new_child(&mut m, "delta", ElemVal::Bool(false)).unwrap();

    m.remove(l).unwrap();

    println!("{m:?}");
}
