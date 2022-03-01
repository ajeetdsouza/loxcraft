fn main() {
    lalrpop::Configuration::new()
        .use_cargo_dir_conventions()
        .process_file("src/syntax/grammar.lalrpop")
        .unwrap();
}
