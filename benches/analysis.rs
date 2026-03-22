use code_analyze_mcp::types::SymbolMatchMode;
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
                SymbolMatchMode::Exact,
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

fn subtree_count_overhead(c: &mut Criterion) {
    use std::fs;
    use tempfile::TempDir;

    // Create fixture: root/ with 3 levels and 120 files (5 * 4 * 6)
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    for i in 0..5usize {
        for j in 0..4usize {
            let subsub = root.join(format!("sub{}", i)).join(format!("subsub{}", j));
            fs::create_dir_all(&subsub).unwrap();
            for k in 0..6usize {
                fs::write(subsub.join(format!("file{}.rs", k)), b"fn main() {}").unwrap();
            }
        }
    }

    let mut group = c.benchmark_group("subtree_count_overhead");
    group.sample_size(10);

    group.bench_function("baseline_walk_only", |b| {
        b.iter(|| {
            let entries = code_analyze_mcp::traversal::walk_directory(
                std::hint::black_box(root),
                std::hint::black_box(Some(2)),
            )
            .unwrap();
            std::hint::black_box(entries)
        })
    });

    group.bench_function("with_count_files_by_dir", |b| {
        b.iter(|| {
            let entries = code_analyze_mcp::traversal::walk_directory(
                std::hint::black_box(root),
                std::hint::black_box(Some(2)),
            )
            .unwrap();
            let counts =
                code_analyze_mcp::traversal::count_files_by_dir(std::hint::black_box(root))
                    .unwrap();
            std::hint::black_box((entries, counts))
        })
    });

    group.finish();
    // Keep dir alive until benchmarks are done
    drop(dir);
}

criterion_group!(
    benches,
    overview_benchmark,
    file_details_benchmark,
    symbol_focus_benchmark,
    subtree_count_overhead
);
criterion_main!(benches);
