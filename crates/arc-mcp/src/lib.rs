// SPDX-License-Identifier: MIT
#![forbid(unsafe_code)]
//! ARC MCP — Model Context Protocol client and tool integration.
//!
//! Provides a JSON-RPC 2.0 stdio client for communicating with MCP servers
//! like codebase-memory-mcp, plus typed wrappers for structural code
//! intelligence tools.

pub mod client;
pub mod security;
pub mod tools;
