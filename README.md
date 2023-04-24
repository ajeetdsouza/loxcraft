# loxcraft

Language tooling for the [Lox programming language], created by [Bob Nystrom].

## Features

- [x] Bytecode compiler + garbage collected runtime
- [x] Online playground ([try it out!][lox playground])
- [x] REPL
- [x] Syntax highlighting, via [tree-sitter-lox]
- [x] IDE integration, via the [Language Server Protocol]

## Screenshots

![Screenshot of REPL]

![Screenshot of online playground]

## Benchmarks

Time taken to execute the [benchmark suite] (lower is better):

| Benchmark         | loxcraft | clox   | jlox    |
| ----------------- | -------- | ------ | ------- |
| binary_tree       | 8.29s    | 8.13s  | 26.41s  |
| equality_1        | 7.17s    | 7.73s  | 10.01s  |
| equality_2        | 8.39s    | 9.66s  | 14.30s  |
| fib               | 10.90s   | 10.09s | 21.89s  |
| instantiation     | 10.83s   | 12.84s | 14.24s  |
| invocation        | 9.93s    | 8.93s  | 15.77s  |
| method_call       | 11.01s   | 9.12s  | 62.03s  |
| properties        | 10.05s   | 5.98s  | 69.77s  |
| string_equality_1 | 7.76s    | 7.66s  | 34.08s  |
| string_equality_2 | 10.78s   | 10.52s | 36.25s  |
| trees             | 9.97s    | 8.72s  | 72.87s  |
| zoo               | 10.67s   | 6.18s  | 100.10s |

![Benchmarks]

Benchmarks were run with the following configuration:

- Device: Apple MacBook Pro (16-inch, 2021)
- Processor: M1 Pro
- RAM: 16 GiB
- OS: macOS Ventura 13.2
- Rust: 1.66.1
- Apple Clang: 14.0.0
- Oracle JDK: 19.0.2

[benchmarks]: https://user-images.githubusercontent.com/1777663/216903842-5d626770-e599-491e-8e09-83b2f956cf34.svg
[benchmark suite]: https://github.com/ajeetdsouza/loxcraft/tree/main/res/benchmarks
[bob nystrom]: https://github.com/munificent
[language server protocol]: https://microsoft.github.io/language-server-protocol/
[lox playground]: https://ajeetdsouza.github.io/loxcraft/
[lox programming language]: http://craftinginterpreters.com/
[screenshot of online playground]: https://user-images.githubusercontent.com/1777663/201918922-39b567fe-9375-4990-8224-e540cf3266bc.png
[screenshot of repl]: https://user-images.githubusercontent.com/1777663/216910834-4ea40427-34d7-43e0-8ba0-06638dfb0fa2.png
[tree-sitter-lox]: https://github.com/ajeetdsouza/tree-sitter-lox
