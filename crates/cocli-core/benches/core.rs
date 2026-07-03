use cocli_core::{CocliConfig, SessionSummary, find_session};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_config_load(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[ui]\ncolor = 'auto'\nbell = false\n").unwrap();
    c.bench_function("config_load", |b| {
        b.iter(|| CocliConfig::load(black_box(&path)).unwrap())
    });
}

fn bench_session_lookup(c: &mut Criterion) {
    let sessions: Vec<_> = (0..10_000)
        .map(|i| SessionSummary {
            id: format!("id-{i}"),
            name: format!("session-{i}"),
        })
        .collect();
    c.bench_function("compact_session_lookup_10k", |b| {
        b.iter(|| find_session(black_box(&sessions), black_box("session-9999")))
    });
}

criterion_group!(benches, bench_config_load, bench_session_lookup);
criterion_main!(benches);
