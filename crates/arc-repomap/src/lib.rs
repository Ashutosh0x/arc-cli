// SPDX-License-Identifier: MIT
use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
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

        let entries: Vec<_> = WalkBuilder::new(&self.root)
            .hidden(true)
            .git_ignore(true)
            .build()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .collect();

        // Process files entirely in parallel! (Rayon)
        let mut results: Vec<String> = entries
            .into_par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str())?;

                let (language, query_source) = match ext {
                    "rs" => (
                        tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (function_item name: (identifier) @name) @function.def
                        (struct_item name: (type_identifier) @name) @struct.def
                        (trait_item name: (type_identifier) @name) @trait.def
                        (impl_item type: (type_identifier) @name) @impl.def
                    "#,
                    ),
                    "py" => (
                        tree_sitter_python::LANGUAGE.into(),
                        r#"
                        (function_definition name: (identifier) @name) @function.def
                        (class_definition name: (identifier) @name) @class.def
                    "#,
                    ),
                    "ts" | "tsx" => (
                        tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (function_declaration name: (identifier) @name) @function.def
                        (class_declaration name: (type_identifier) @name) @class.def
                        (interface_declaration name: (type_identifier) @name) @interface.def
                    "#,
                    ),
                    "go" => (
                        tree_sitter_go::LANGUAGE.into(),
                        r#"
                        (function_declaration name: (identifier) @name) @function.def
                        (method_declaration name: (field_identifier) @name) @method.def
                        (type_declaration (type_spec name: (type_identifier) @name)) @type.def
                    "#,
                    ),
                    "cpp" | "cc" | "h" | "hpp" => (
                        tree_sitter_cpp::LANGUAGE.into(),
                        r#"
                        (function_definition declarator: (_) @name) @function.def
                        (class_specifier name: (type_identifier) @name) @class.def
                    "#,
                    ),
                    _ => return None,
                };

                let content = std::fs::read_to_string(path).ok()?;
                let language_obj: tree_sitter::Language = language;

                let mut parser = Parser::new();
                parser.set_language(&language_obj).ok()?;

                let tree = parser.parse(&content, None)?;
                let query = Query::new(&language_obj, query_source).ok()?;
                let mut cursor = QueryCursor::new();

                let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
                let mut file_decls = Vec::new();

                for m in matches {
                    for capture in m.captures {
                        if capture.index == 0 {
                            let text = capture.node.utf8_text(content.as_bytes()).unwrap_or("");
                            if let Some(first_line) = text.lines().next() {
                                file_decls
                                    .push(first_line.trim_end_matches('{').trim().to_string());
                            }
                        }
                    }
                }

                if file_decls.is_empty() {
                    return None;
                }

                file_decls.dedup();
                let relative_path = path.strip_prefix(&self.root).unwrap_or(path);
                let mut block = format!("\nFile: {}\n", relative_path.display());
                for decl in file_decls {
                    block.push_str(&format!("  {} ...\n", decl));
                }
                Some(block)
            })
            .collect();

        // Deterministic output
        results.sort();
        for r in results {
            map_output.push_str(&r);
        }

        Ok(map_output)
    }
}
