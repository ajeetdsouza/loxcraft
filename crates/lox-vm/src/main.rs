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

    let now = std::time::SystemTime::now();
    let mut intern = Intern::default();
    let chunk = Compiler::compile(
        r#"
        var a = 1 + "asdf";
    "#,
        &mut intern,
    );
    VM::new().run(&chunk, &mut intern).unwrap();
    println!("elapsed: {}", now.elapsed().unwrap().as_millis());
}
