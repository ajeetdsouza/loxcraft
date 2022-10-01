use lox_vm::VM;

fn main() {
    let mut stdout = std::io::stdout().lock();
    VM::default()
        .run(
            r#"
            fun fib(n) {
              if (n < 2) return n;
              return fib(n - 2) + fib(n - 1);
            }

            var start = clock();
            print fib(35) == 9227465;
            print clock() - start;
    "#,
            &mut stdout,
        )
        .unwrap();
}
