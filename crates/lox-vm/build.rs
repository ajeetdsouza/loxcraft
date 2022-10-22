fn main() {
    if cfg!(test) {
        build_deps::rerun_if_changed_paths("../../benchmarks/**/*.lox").unwrap();
        build_deps::rerun_if_changed_paths("../../examples/**/*.lox").unwrap();
    }
}
