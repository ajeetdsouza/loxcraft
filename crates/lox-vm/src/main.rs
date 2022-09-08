#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::collections::HashSet;

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
        print "234"+"1234";
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
