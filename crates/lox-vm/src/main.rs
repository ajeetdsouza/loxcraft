#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use crate::intern::Intern;

mod chunk;
mod compiler;
mod intern;
mod op;
mod value;
mod vm;

fn main() {
    use compiler::Compiler;
    use vm::VM;

    let mut intern = Intern::default();

    let chunk = Compiler::compile(
        r#"
        var a = "foo";
        var b = "bar";
        print a + b;
        print a == b;
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
