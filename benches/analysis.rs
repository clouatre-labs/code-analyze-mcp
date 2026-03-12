use criterion::{Criterion, criterion_group, criterion_main};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio_util::sync::CancellationToken;

fn overview_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("overview");
    group.sample_size(10);

    group.bench_function("analyze_directory_src", |b| {
        b.iter(|| {
            let path = std::hint::black_box(Path::new("src"));
            let entries = code_analyze_mcp::traversal::walk_directory(path, None).unwrap();
            let progress = Arc::new(AtomicUsize::new(0));
            let ct = CancellationToken::new();

            code_analyze_mcp::analyze::analyze_directory_with_progress(path, entries, progress, ct)
        });
    });

    group.finish();
}

fn file_details_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_details");
    group.sample_size(10);

    group.bench_function("analyze_file_lib_rs", |b| {
        b.iter(|| {
            let path = std::hint::black_box("src/lib.rs");
            let ast_recursion_limit = std::hint::black_box(None);

            code_analyze_mcp::analyze::analyze_file(path, ast_recursion_limit)
        });
    });

    group.finish();
}

fn symbol_focus_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_focus");
    group.sample_size(10);

    group.bench_function("analyze_focused_src", |b| {
        b.iter(|| {
            let path = std::hint::black_box(Path::new("src"));
            let focus = std::hint::black_box("analyze_directory");
            let follow_depth = std::hint::black_box(2);
            let max_depth = std::hint::black_box(None);
            let ast_recursion_limit = std::hint::black_box(None);
            let progress = Arc::new(AtomicUsize::new(0));
            let ct = CancellationToken::new();

            code_analyze_mcp::analyze::analyze_focused_with_progress(
                path,
                focus,
                follow_depth,
                max_depth,
                ast_recursion_limit,
                progress,
                ct,
                false,
            )
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    overview_benchmark,
    file_details_benchmark,
    symbol_focus_benchmark
);
criterion_main!(benches);
