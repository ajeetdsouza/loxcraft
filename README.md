<!-- markdownlint-configure-file {
  "MD033": false,
  "MD041": false
} -->

<div align="center">

# loxcraft

[![crates.io](https://img.shields.io/crates/v/loxcraft)](https://crates.io/crates/loxcraft)

**Language tooling** for the **[Lox programming language](http://craftinginterpreters.com/)**.

</div>

## Installation

```sh
cargo install loxcraft --locked
```

## Features

- [x] Bytecode compiler + garbage collected runtime
- [x] Online playground, via WebAssembly ([try it out!](https://ajeetdsouza.github.io/loxcraft/))
- [x] REPL
- [x] Syntax highlighting, via [tree-sitter-lox](https://github.com/ajeetdsouza/tree-sitter-lox)
- [x] IDE integration, via the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

## Screenshots

![Screenshot of REPL](https://user-images.githubusercontent.com/1777663/216910834-4ea40427-34d7-43e0-8ba0-06638dfb0fa2.png)

![Screenshot of online playground](https://user-images.githubusercontent.com/1777663/201918922-39b567fe-9375-4990-8224-e540cf3266bc.png)

## Benchmarks

Time taken to execute the [benchmark suite](https://github.com/ajeetdsouza/loxcraft/tree/main/res/benchmarks) (lower is better):

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

![Benchmarks](https://user-images.githubusercontent.com/1777663/216903842-5d626770-e599-491e-8e09-83b2f956cf34.svg)

Benchmarks were run with the following configuration:

- Device: Apple MacBook Pro (16-inch, 2021)
- Processor: M1 Pro
- RAM: 16 GiB
- OS: macOS Ventura 13.2
- Rust: 1.66.1
- Apple Clang: 14.0.0
- Oracle JDK: 19.0.2

## References

So you want to build your own programming language! Here's some extremely helpful resources I referred to when building `loxcraft`:

- [Crafting Interpreters](https://craftinginterpreters.com/) by Bob Nystrom: this book introduces you to a teaching programming language named Lox, walks you through implementing a full-featured tree walking interpreter for in in Java, and then shows you how to build a bytecode compiler + VM for it in C. I cannot recommend this book enough.
- Bob Nystrom also has a [blog](https://journal.stuffwithstuff.com/), and his articles are really well written (see his post on [Pratt parsers](https://journal.stuffwithstuff.com/2011/03/19/pratt-parsers-expression-parsing-made-easy/) / [garbage collectors](https://journal.stuffwithstuff.com/2013/12/08/babys-first-garbage-collector/)). I'd also recommend going through the source code for [Wren](https://wren.io/), it shares a lot of code with Lox. Despite the deceptive simplicity of the implementation, it (like Lox) is incredibly fast - it's a great way to learn how to build production grade compilers in general.
- [Writing an Interpreter in Go](https://interpreterbook.com/) / [Writing a Compiler in Go](https://compilerbook.com/) by Thorsten Ball is a great set of books. Since it uses Go, it piggybacks on Go's garbage collector instead of building one of its own. This simplifies the implementation, making this book a lot easier to grok - but it also means that you may have trouble porting it to a non-GC language (like Rust).
- [Make a Language](https://lunacookies.github.io/lang/) by Luna Razzaghipour is a fantastic series. Notably, this book constructs its syntax tree using the same library used by [rust-analyzer](https://rust-analyzer.github.io/) ([rowan](https://github.com/rust-analyzer/rowan)).
- [Simple but Powerful Pratt Parsing](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html) by Alex Kladov (one of the main authors behind rust-analyzer) is a great tutorial on building a parser in Rust. The rest of his blog is incredible too!
- [rust-langdev](https://github.com/Kixiron/rust-langdev) has a lot of libraries for building compilers in Rust. To start off, I'd suggest [logos](https://github.com/maciejhirsz/logos) for lexing, [LALRPOP](https://lalrpop.github.io/lalrpop/) / [chumsky](https://github.com/zesterer/chumsky) for parsing, and [rust-gc](https://github.com/Manishearth/rust-gc) for garbage collection.
- [Learning Rust with Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/) is a quick tutorial on unsafe Rust, which you'll need if you're building a garbage collector yourself.
- If you want some inspiration for a production-grade language built in Rust, you might want to go through the source code of [Starlark](https://github.com/facebook/starlark-rust) and [Gluon](https://github.com/gluon-lang/gluon).

## Contributors

- [Ajeet D'Souza](https://github.com/ajeetdsouza)
- [Kartik Sharma](https://github.com/crazystylus)
