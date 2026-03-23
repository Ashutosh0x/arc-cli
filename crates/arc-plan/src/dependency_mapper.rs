// SPDX-License-Identifier: MIT
use crate::plan_model::{DependencyEdge, DependencyGraph, DependencyNode, EdgeType, NodeType};
use crate::read_only_tools::{FileDependencies, ReadOnlyToolSet};
use anyhow::Result;
use std::collections::HashMap;
use tracing::instrument;

/// Builds a complete dependency graph of the codebase by analyzing
/// imports, exports, and module relationships.
pub struct DependencyMapper<'a> {
    tools: &'a ReadOnlyToolSet,
}

impl<'a> DependencyMapper<'a> {
    pub fn new(tools: &'a ReadOnlyToolSet) -> Self {
        Self { tools }
    }

    /// Scan the entire codebase and build a dependency graph.
    #[instrument(skip(self))]
    pub async fn build_graph(&self) -> Result<DependencyGraph> {
        // Find all Rust source files
        let rust_files = self.tools.glob_files("**/*.rs").await?;

        // Analyze each file
        let mut all_deps: Vec<FileDependencies> = Vec::with_capacity(rust_files.len());
        for file in &rust_files {
            match self.tools.analyze_rust_deps(file).await {
                Ok(deps) => all_deps.push(deps),
                Err(e) => {
                    tracing::warn!("Failed to analyze {}: {}", file.display(), e);
                },
            }
        }

        // Build nodes
        let nodes: Vec<DependencyNode> = all_deps
            .iter()
            .map(|dep| {
                let node_type = if dep.path.to_string_lossy().contains("test") {
                    NodeType::Test
                } else if !dep.pub_traits.is_empty() {
                    NodeType::Trait
                } else if !dep.pub_structs.is_empty() {
                    NodeType::Struct
                } else {
                    NodeType::Module
                };

                DependencyNode {
                    file_path: dep.path.to_string_lossy().to_string(),
                    node_type,
                    imports: dep.imports.clone(),
                    exports: dep
                        .pub_functions
                        .iter()
                        .chain(&dep.pub_structs)
                        .chain(&dep.pub_traits)
                        .cloned()
                        .collect(),
                    loc: dep.loc,
                }
            })
            .collect();

        // Build export map: symbol -> file_path
        let mut export_map: HashMap<String, String> = HashMap::new();
        for node in &nodes {
            for export in &node.exports {
                export_map.insert(export.clone(), node.file_path.clone());
            }
        }

        // Build edges based on import resolution
        let mut edges: Vec<DependencyEdge> = Vec::new();
        for dep in &all_deps {
            let from = dep.path.to_string_lossy().to_string();
            for import in &dep.imports {
                // Try to resolve the import to a file
                let symbol = import.split("::").last().unwrap_or(import);
                if let Some(to) = export_map.get(symbol) {
                    if to != &from {
                        edges.push(DependencyEdge {
                            from: from.clone(),
                            to: to.clone(),
                            edge_type: EdgeType::Import,
                        });
                    }
                }
            }

            // Impl edges
            for impl_name in &dep.impls {
                if let Some(to) = export_map.get(impl_name) {
                    if to != &from {
                        edges.push(DependencyEdge {
                            from: from.clone(),
                            to: to.clone(),
                            edge_type: EdgeType::Implements,
                        });
                    }
                }
            }
        }

        Ok(DependencyGraph { nodes, edges })
    }

    /// Find all files that would be affected by changes to a given file.
    pub fn impact_analysis(&self, graph: &DependencyGraph, changed_file: &str) -> Vec<String> {
        let mut affected = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back(changed_file.to_string());
        visited.insert(changed_file.to_string());

        while let Some(current) = queue.pop_front() {
            // Find all files that import from current
            for edge in &graph.edges {
                if edge.to == current && !visited.contains(&edge.from) {
                    visited.insert(edge.from.clone());
                    affected.push(edge.from.clone());
                    queue.push_back(edge.from.clone());
                }
            }
        }

        affected
    }
}
