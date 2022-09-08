#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod chunk;
mod compiler;
mod intern;
mod op;
mod value;
mod vm;

fn main() {
    use compiler::Compiler;
    use intern::Intern;
    use vm::VM;

    let mut intern = Intern::default();

    let chunk = Compiler::compile(
        r#"
        var a = "foo";
        var b = "bar";
        print a + b;
        print a + "bar" == "foo" + b;
    "#,
        &mut intern,
    );
    chunk.debug("chunk");

    // let now = std::time::SystemTime::now();

    for _ in 0..1 {
        VM::new().run(&chunk, &mut intern);
    }
    // println!("elapsed: {}", now.elapsed().unwrap().as_millis());
}
