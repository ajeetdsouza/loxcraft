fn main() {
    lalrpop::process_root().unwrap();
    if cfg!(test) {
        for path in ["res/examples/**/*.lox", "res/benchmarks/**/*.lox"] {
            build_deps::rerun_if_changed_paths(path).expect("could not read path");
        }
    }
}
