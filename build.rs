fn main() {
    lalrpop::Configuration::new()
        .use_cargo_dir_conventions()
        .process_file("res/grammar.lalrpop")
        .expect("failed to process LALRPOP grammar");

    if cfg!(test) {
        for path in ["res/examples/**/*.lox", "res/benchmarks/**/*.lox"] {
            build_deps::rerun_if_changed_paths(path).expect("could not read path");
        }
    }
}
