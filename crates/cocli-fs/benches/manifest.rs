use cocli_fs::{FileManifest, HashMode, ManifestOptions, chunk_plan, diff};
use cocli_protocol::FileEntry;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_manifest_diff(c: &mut Criterion) {
    let local = manifest(10_000, 10);
    let remote = manifest(10_000, 11);
    c.bench_function("file_manifest_diff_10k", |b| {
        b.iter(|| diff(black_box(&local), black_box(&remote), black_box(false)))
    });
}

fn bench_chunk_plan(c: &mut Criterion) {
    c.bench_function("remote_file_chunk_planner_1gib", |b| {
        b.iter(|| chunk_plan(black_box(1024 * 1024 * 1024), black_box(8 * 1024 * 1024)))
    });
}

fn bench_manifest_build(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..100 {
        std::fs::write(dir.path().join(format!("{i}.txt")), b"x").unwrap();
    }
    let options = ManifestOptions {
        hash: HashMode::Never,
        ..ManifestOptions::default()
    };
    c.bench_function("local_manifest_build_100_no_hash", |b| {
        b.iter(|| FileManifest::build(black_box(dir.path()), black_box(&options)).unwrap())
    });
}

fn manifest(n: usize, mtime: u64) -> FileManifest {
    FileManifest {
        entries: (0..n)
            .map(|i| FileEntry {
                path: format!("src/{i}.py"),
                size: i as u64,
                mtime_unix: mtime,
                executable: false,
                hash: None,
            })
            .collect(),
    }
}

criterion_group!(
    benches,
    bench_manifest_diff,
    bench_chunk_plan,
    bench_manifest_build
);
criterion_main!(benches);
