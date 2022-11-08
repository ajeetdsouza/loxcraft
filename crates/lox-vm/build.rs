fn main() {
    if cfg!(test) {
        for path in ["../../examples/**/*.lox", "../../benchmarks/**/*.lox"] {
            build_deps::rerun_if_changed_paths(path).expect("could not read path");
        }
    }
}
