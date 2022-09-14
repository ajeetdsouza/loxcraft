use criterion::{criterion_group, criterion_main, Criterion};

use std::fs;
use std::path::PathBuf;

pub fn lang(c: &mut Criterion) {
    for entry in fs::read_dir("benches/lang").unwrap() {
        let path = PathBuf::from(entry.unwrap().file_name());
        let _source = fs::read_to_string(&path)
            .unwrap_or_else(|_| format!("could not read test file: {}", path.display()));
        c.bench_function(path.to_str().unwrap(), |b| b.iter(|| todo!()));
    }
}

criterion_group!(benches, lang);
criterion_main!(benches);
