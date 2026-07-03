use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use uuid::Uuid;

use clap::Parser;
use colab_cli::cocli::cli::args::Cli;
use colab_cli::cocli::session::client::{build_assign_url, strip_xssi, uuid_to_websafe_base64};
use colab_cli::cocli::session::model::{Shape, Variant};
use colab_cli::cocli::ui::ccu_rate;

fn bench_strip_xssi(c: &mut Criterion) {
    let mut g = c.benchmark_group("strip_xssi");

    let with_prefix = ")]}'\n{\"endpoint\":\"abc-123\",\"variant\":\"GPU\",\"accelerator\":\"T4\"}";
    let without_prefix = "{\"endpoint\":\"abc-123\",\"variant\":\"GPU\",\"accelerator\":\"T4\"}";

    g.bench_function("with_prefix", |b| {
        b.iter(|| strip_xssi(black_box(with_prefix)))
    });
    g.bench_function("without_prefix", |b| {
        b.iter(|| strip_xssi(black_box(without_prefix)))
    });
    g.bench_function("empty", |b| b.iter(|| strip_xssi(black_box(""))));

    g.finish();
}

fn bench_uuid_encoding(c: &mut Criterion) {
    let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    c.bench_function("uuid_to_websafe_base64", |b| {
        b.iter(|| uuid_to_websafe_base64(black_box(id)))
    });
}

fn bench_build_assign_url(c: &mut Criterion) {
    let mut g = c.benchmark_group("build_assign_url");
    let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let domain = "https://colab.research.google.com";

    g.bench_function("cpu_standard", |b| {
        b.iter(|| {
            build_assign_url(
                black_box(domain),
                black_box(id),
                black_box(Variant::Cpu),
                black_box(None),
                black_box(Shape::Standard),
            )
        })
    });

    g.bench_function("gpu_t4_highmem", |b| {
        b.iter(|| {
            build_assign_url(
                black_box(domain),
                black_box(id),
                black_box(Variant::Gpu),
                black_box(Some("T4")),
                black_box(Shape::HighMem),
            )
        })
    });

    g.bench_function("tpu_standard", |b| {
        b.iter(|| {
            build_assign_url(
                black_box(domain),
                black_box(id),
                black_box(Variant::Tpu),
                black_box(None),
                black_box(Shape::Standard),
            )
        })
    });

    g.finish();
}

fn bench_variant_serde(c: &mut Criterion) {
    let mut g = c.benchmark_group("variant_serde");

    g.bench_function("deserialize_string", |b| {
        b.iter(|| {
            let _: Variant = serde_json::from_str(black_box("\"GPU\"")).unwrap();
        })
    });
    g.bench_function("deserialize_int", |b| {
        b.iter(|| {
            let _: Variant = serde_json::from_str(black_box("1")).unwrap();
        })
    });
    g.bench_function("serialize", |b| {
        b.iter_batched(
            || Variant::Gpu,
            |v| serde_json::to_string(&v).unwrap(),
            BatchSize::SmallInput,
        )
    });

    g.finish();
}

fn bench_shape_roundtrip(c: &mut Criterion) {
    let mut g = c.benchmark_group("shape_serde");

    g.bench_function("deserialize", |b| {
        b.iter(|| {
            let _: Shape = serde_json::from_str(black_box("1")).unwrap();
        })
    });
    g.bench_function("serialize", |b| {
        b.iter_batched(
            || Shape::HighMem,
            |s| serde_json::to_string(&s).unwrap(),
            BatchSize::SmallInput,
        )
    });

    g.finish();
}

fn bench_ccu_rate_lookup(c: &mut Criterion) {
    c.bench_function("ccu_rate_gpu_t4", |b| {
        b.iter(|| ccu_rate(black_box("GPU"), black_box("T4")))
    });
}

fn bench_command_parse_smoke(c: &mut Criterion) {
    c.bench_function("command_parse_smoke", |b| {
        b.iter(|| {
            Cli::try_parse_from(black_box([
                "colab-cli",
                "exec",
                "run",
                "train.py",
                "--session",
                "trainer",
                "--",
                "--epochs",
                "3",
            ]))
            .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_strip_xssi,
    bench_uuid_encoding,
    bench_build_assign_url,
    bench_variant_serde,
    bench_shape_roundtrip,
    bench_ccu_rate_lookup,
    bench_command_parse_smoke,
);
criterion_main!(benches);
