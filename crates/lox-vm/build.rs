fn main() {
    println!("cargo:rerun-if-changed=../../benchmarks/");
    println!("cargo:rerun-if-changed=../../examples/");
}
