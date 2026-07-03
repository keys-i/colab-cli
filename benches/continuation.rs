use colab_cli::cocli::r#continue::manifest::{ContinuationManifest, ExecutionStep};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_continuation_json(c: &mut Criterion) {
    let mut manifest = ContinuationManifest::new("2026-07-03T00:00:00Z", "trainer");
    for i in 0..100 {
        manifest.pending_steps.push(ExecutionStep {
            id: format!("step-{i}"),
            command: vec![
                "python".into(),
                "train.py".into(),
                "--epoch".into(),
                i.to_string(),
            ],
            cwd: Some("/content".into()),
        });
    }
    let json = manifest.to_json_pretty().unwrap();

    c.bench_function("continuation_manifest_serialize", |b| {
        b.iter(|| black_box(&manifest).to_json_pretty().unwrap())
    });
    c.bench_function("continuation_manifest_deserialize", |b| {
        b.iter(|| ContinuationManifest::from_json(black_box(json.as_bytes())).unwrap())
    });
}

criterion_group!(benches, bench_continuation_json);
criterion_main!(benches);
