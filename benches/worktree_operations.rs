//! Git worktree operation benchmarks.
//! Measures the overhead ARC adds on top of raw git operations.

use criterion::{criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

fn setup_git_repo(dir: &TempDir) {
    let path = dir.path();
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();

    // Create some files
    for i in 0..20 {
        std::fs::write(
            path.join(format!("file_{i}.rs")),
            format!("pub fn func_{i}() -> i32 {{ {i} }}\n"),
        )
        .unwrap();
    }

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(path)
        .output()
        .unwrap();
}

fn bench_worktree_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("worktree_operations");
    group.sample_size(10);

    group.bench_function("create_and_remove_worktree", |b| {
        let dir = TempDir::new().unwrap();
        setup_git_repo(&dir);

        let mut counter = 0u64;
        b.iter(|| {
            counter += 1;
            let wt_path = dir
                .path()
                .join(format!(".arc-worktrees/bench-{counter}"));
            let branch = format!("bench-branch-{counter}");

            // Create
            let output = std::process::Command::new("git")
                .args([
                    "worktree",
                    "add",
                    "-b",
                    &branch,
                    &wt_path.display().to_string(),
                    "HEAD",
                ])
                .current_dir(dir.path())
                .output()
                .unwrap();
            assert!(output.status.success());

            // Remove
            let output = std::process::Command::new("git")
                .args([
                    "worktree",
                    "remove",
                    "--force",
                    &wt_path.display().to_string(),
                ])
                .current_dir(dir.path())
                .output()
                .unwrap();

            // Cleanup branch
            let _ = std::process::Command::new("git")
                .args(["branch", "-D", &branch])
                .current_dir(dir.path())
                .output();

            criterion::black_box(output.status.success());
        });
    });

    group.bench_function("sparse_checkout_setup", |b| {
        let dir = TempDir::new().unwrap();
        setup_git_repo(&dir);

        // Create subdirectories
        std::fs::create_dir_all(dir.path().join("crates/core")).unwrap();
        std::fs::create_dir_all(dir.path().join("crates/cli")).unwrap();
        std::fs::create_dir_all(dir.path().join("docs")).unwrap();
        std::fs::write(dir.path().join("crates/core/lib.rs"), "pub fn core() {}").unwrap();
        std::fs::write(dir.path().join("crates/cli/main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("docs/README.md"), "# Docs").unwrap();

        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "add dirs"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let mut counter = 0u64;
        b.iter(|| {
            counter += 1;
            let wt_path = dir
                .path()
                .join(format!(".arc-worktrees/sparse-{counter}"));
            let branch = format!("sparse-branch-{counter}");

            // Create worktree without checkout
            std::process::Command::new("git")
                .args([
                    "worktree",
                    "add",
                    "--no-checkout",
                    "-b",
                    &branch,
                    &wt_path.display().to_string(),
                    "HEAD",
                ])
                .current_dir(dir.path())
                .output()
                .unwrap();

            // Setup sparse checkout
            std::process::Command::new("git")
                .args(["sparse-checkout", "init", "--cone"])
                .current_dir(&wt_path)
                .output()
                .unwrap();

            std::process::Command::new("git")
                .args(["sparse-checkout", "set", "crates/core"])
                .current_dir(&wt_path)
                .output()
                .unwrap();

            // Cleanup
            let _ = std::process::Command::new("git")
                .args(["worktree", "remove", "--force", &wt_path.display().to_string()])
                .current_dir(dir.path())
                .output();
            let _ = std::process::Command::new("git")
                .args(["branch", "-D", &branch])
                .current_dir(dir.path())
                .output();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_worktree_ops);
criterion_main!(benches);
