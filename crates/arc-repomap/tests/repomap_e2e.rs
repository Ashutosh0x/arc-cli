use arc_repomap::RepoMap;
use std::path::PathBuf;

#[test]
fn evaluate_repomap_token_reduction() {
    let cargo_manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    // Test target: The source code of `arc-repomap` itself.
    // It is a real rust project structure containing implementations and imports.
    let src_dir = PathBuf::from(&cargo_manifest_dir).join("src");

    // 1. Calculate baseline raw string length
    let full_content = std::fs::read_to_string(src_dir.join("lib.rs"))
        .expect("Failed to read src/lib.rs for baseline calculation");
    let initial_character_count = full_content.len();

    // 2. Generate structured repomap
    let repomap = RepoMap::new(&src_dir);
    let map_output = repomap.generate_map().expect("Failed to generate repomap");
    let compressed_character_count = map_output.len();

    println!("=== ARC Repomap Empirical Evaluation ===");
    println!("Target Directory: {}", src_dir.display());
    println!(
        "Naive Full File Context Size: {} characters",
        initial_character_count
    );
    println!(
        "Repomap Compressed Context Size: {} characters",
        compressed_character_count
    );

    let reduction_percentage =
        100.0 - ((compressed_character_count as f64 / initial_character_count as f64) * 100.0);
    println!("Reduction Ratio: {:.2}%", reduction_percentage);

    assert!(
        compressed_character_count < initial_character_count,
        "Repomap must strictly reduce token limits over raw source arrays"
    );

    // Validate AST extracted specific structural definitions accurately.
    assert!(
        map_output.contains("pub struct RepoMap"),
        "AST structural definition missing: 'pub struct RepoMap'"
    );
    assert!(
        map_output.contains("pub fn generate_map"),
        "AST method signature missing: 'pub fn generate_map'"
    );
}
