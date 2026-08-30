use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;
use std::path::PathBuf;
use warp_lib::{dir_stats, to_long_path};

fn make_fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!("warp_bench_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let payload = vec![0xA5u8; 16 * 1024];
    for d in 0..30 {
        let dir = root.join(format!("d{d:03}"));
        fs::create_dir_all(&dir).unwrap();
        for f in 0..300 {
            fs::write(dir.join(format!("f{f:04}.bin")), &payload).unwrap();
        }
    }
    root
}

fn bench_scan(c: &mut Criterion) {
    let root = make_fixture();
    let s = root.to_string_lossy().to_string();
    c.bench_function("scan 9k files", |b| b.iter(|| dir_stats(black_box(&s))));
    let _ = fs::remove_dir_all(&root);
}

fn bench_to_long_path(c: &mut Criterion) {
    c.bench_function("to_long_path", |b| {
        b.iter(|| to_long_path(black_box(r"C:\very\long\path\file.txt")))
    });
}

criterion_group!(benches, bench_scan, bench_to_long_path);
criterion_main!(benches);
