use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, Query, QueryCursor};

/// Repomap builds a compressed structural overview of a repository's code,
/// extracting method signatures, structs, and traits without the function bodies
/// to save massive amounts of LLM context window tokens safely.
pub struct RepoMap {
    root: PathBuf,
}

impl RepoMap {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Recursively walks the repository, parses Rust files, and generates a unified map string.
    pub fn generate_map(&self) -> Result<String> {
        let mut map_output = String::new();
        map_output.push_str("=== Repository Structural Map ===\n");

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)?;

        // A tree-sitter query to extract fundamental declarations
        let query_source = r#"
            (function_item name: (identifier) @name) @function.def
            (struct_item name: (type_identifier) @name) @struct.def
            (trait_item name: (type_identifier) @name) @trait.def
            (impl_item type: (type_identifier) @name) @impl.def
        "#;
        
        let query = Query::new(language, query_source)?;
        let mut cursor = QueryCursor::new();

        let walker = WalkBuilder::new(&self.root)
            .hidden(true)
            .git_ignore(true)
            .build();

        for result in walker {
            match result {
                Ok(entry) if entry.path().is_file() => {
                    if entry.path().extension().and_then(|e| e.to_str()) == Some("rs") {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            let tree = match parser.parse(&content, None) {
                                Some(t) => t,
                                None => continue,
                            };

                            let relative_path = entry.path().strip_prefix(&self.root).unwrap_or(entry.path());
                            let mut file_decls = Vec::new();

                            let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
                            for m in matches {
                                for capture in m.captures {
                                    if capture.index == 0 { // Just taking the full nodes (simplified)
                                        let text = capture.node.utf8_text(content.as_bytes()).unwrap_or("");
                                        // Take just the first line (signature)
                                        if let Some(first_line) = text.lines().next() {
                                            file_decls.push(first_line.trim_end_matches('{').trim().to_string());
                                        }
                                    }
                                }
                            }

                            if !file_decls.is_empty() {
                                file_decls.dedup();
                                map_output.push_str(&format!("\nFile: {}\n", relative_path.display()));
                                for decl in file_decls {
                                    map_output.push_str(&format!("  {} ...\n", decl));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(map_output)
    }
}
