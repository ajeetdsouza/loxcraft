fn main() {
    lalrpop::Configuration::new()
        .use_cargo_dir_conventions()
        .process_file("src/grammar.lalrpop")
        .unwrap();
}
