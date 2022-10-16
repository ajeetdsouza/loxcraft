use lox_vm::VM;

fn main() {
    let mut stdout = std::io::stdout().lock();
    let mut vm = VM::default();
    vm.run(
        r#"
        class Toast {}
        print Toast();
    "#,
        &mut stdout,
    )
    .unwrap();
}
