fn main() {
    const GRAMMAR: &str = "src/syntax/grammar.lalrpop";
    lalrpop::Configuration::new().use_cargo_dir_conventions().process_file(GRAMMAR).unwrap();
    println!("cargo:rerun-if-changed={}", GRAMMAR);
}
