//! Go Module Parser - Parses go.mod and Go source files

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, trace};
use tree_sitter::{Node, Parser};

use crate::index::{Symbol, SymbolKind};

/// Parsed Go module information
#[derive(Debug, Clone)]
pub struct ParsedModule {
    pub path: String,
    pub version: Option<String>,
    pub go_version: String,
    pub symbols: Vec<Symbol>,
    pub files: Vec<PathBuf>,
}

/// Parser for Go modules and source files
pub struct GoModuleParser {
    parser: Parser,
}

impl GoModuleParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_go::LANGUAGE.into();
        parser
            .set_language(&language)
            .expect("Failed to load Go grammar");

        Self { parser }
    }

    /// Parse a go.mod file and extract module information
    pub fn parse_module(&self, go_mod_path: &Path) -> Result<ParsedModule> {
        let content = std::fs::read_to_string(go_mod_path)?;
        let module_dir = go_mod_path
            .parent()
            .context("go.mod has no parent directory")?;

        // Parse go.mod
        let (module_path, go_version) = self.parse_go_mod_content(&content)?;

        debug!("📦 Parsing module: {}", module_path);

        // Find all Go files in the module
        let go_files = self.find_go_files(module_dir)?;

        // Parse symbols from all Go files
        let mut all_symbols = Vec::new();
        for file in &go_files {
            match self.parse_go_file(file, &module_path) {
                Ok(symbols) => all_symbols.extend(symbols),
                Err(e) => trace!("Failed to parse {:?}: {}", file, e),
            }
        }

        Ok(ParsedModule {
            path: module_path,
            version: self.extract_version(&content),
            go_version,
            symbols: all_symbols,
            files: go_files,
        })
    }

    /// Parse go.mod content to extract module path and Go version
    fn parse_go_mod_content(&self, content: &str) -> Result<(String, String)> {
        let mut module_path = String::new();
        let mut go_version = String::from("1.21"); // default

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("module ") {
                module_path = trimmed
                    .strip_prefix("module ")
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_string();
            } else if trimmed.starts_with("go ") {
                go_version = trimmed
                    .strip_prefix("go ")
                    .unwrap_or("1.21")
                    .trim()
                    .to_string();
            }
        }

        if module_path.is_empty() {
            anyhow::bail!("No module path found in go.mod");
        }

        Ok((module_path, go_version))
    }

    /// Extract version from go.mod (if present)
    fn extract_version(&self, content: &str) -> Option<String> {
        // Try to find require block with version
        for line in content.lines() {
            if line.contains(&self.parse_go_mod_content(content).ok()?.0) {
                // This is a workspace go.mod, version might be specified
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return Some(parts[2].trim_matches('"').to_string());
                }
            }
        }
        None
    }

    /// Find all Go files in a directory (excluding vendor, test files)
    fn find_go_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in walkdir::WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip vendor and hidden directories
            if path.to_string_lossy().contains("vendor/") || path.to_string_lossy().contains("/.") {
                continue;
            }

            if path.extension() == Some(std::ffi::OsStr::new("go")) {
                // Skip test files for symbol indexing
                if !path
                    .file_name()
                    .map(|n| n.to_string_lossy().ends_with("_test.go"))
                    .unwrap_or(true)
                {
                    files.push(path.to_path_buf());
                }
            }
        }

        Ok(files)
    }

    /// Parse a single Go file and extract symbols
    fn parse_go_file(&self, path: &Path, module_path: &str) -> Result<Vec<Symbol>> {
        let content = std::fs::read_to_string(path)?;
        let package_name = self.extract_package_name(&content);

        let mut parser = Parser::new();
        let language = tree_sitter_go::LANGUAGE.into();
        parser
            .set_language(&language)
            .expect("Failed to load Go grammar");
        let tree = parser
            .parse(&content, None)
            .context("Failed to parse Go file")?;

        let root = tree.root_node();
        let mut symbols = Vec::new();
        let source = content.as_bytes();

        // Walk the AST and extract symbols
        self.extract_symbols_from_node(&root, source, module_path, &package_name, &mut symbols);

        Ok(symbols)
    }

    /// Extract package name from Go file content
    fn extract_package_name(&self, content: &str) -> String {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("package ") {
                return trimmed
                    .strip_prefix("package ")
                    .unwrap_or("")
                    .trim()
                    .to_string();
            }
        }
        "main".to_string()
    }

    /// Recursively extract symbols from AST nodes
    fn extract_symbols_from_node(
        &self,
        node: &Node,
        source: &[u8],
        module_path: &str,
        package_name: &str,
        symbols: &mut Vec<Symbol>,
    ) {
        match node.kind() {
            "function_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(&name_node, source);
                    let signature = self.extract_function_signature(node, source);

                    symbols.push(Symbol {
                        name,
                        package: module_path.to_string(),
                        package_name: package_name.to_string(),
                        kind: SymbolKind::Function,
                        version: None,
                        import_path: module_path.to_string(),
                        doc: self.extract_doc_comment(node, source),
                        signature: Some(signature),
                    });
                }
            }
            "method_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(&name_node, source);
                    let receiver = self.extract_receiver(node, source);
                    let full_name = format!("{}.{}", receiver, name);

                    symbols.push(Symbol {
                        name: full_name,
                        package: module_path.to_string(),
                        package_name: package_name.to_string(),
                        kind: SymbolKind::Method,
                        version: None,
                        import_path: module_path.to_string(),
                        doc: self.extract_doc_comment(node, source),
                        signature: None,
                    });
                }
            }
            "type_declaration" => {
                // Handle type declarations
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "type_spec" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            let name = self.node_text(&name_node, source);
                            let kind = self.determine_type_kind(&child);

                            symbols.push(Symbol {
                                name,
                                package: module_path.to_string(),
                                package_name: package_name.to_string(),
                                kind,
                                version: None,
                                import_path: module_path.to_string(),
                                doc: self.extract_doc_comment(node, source),
                                signature: None,
                            });
                        }
                    }
                }
            }
            "const_declaration" | "var_declaration" => {
                let kind = if node.kind() == "const_declaration" {
                    SymbolKind::Const
                } else {
                    SymbolKind::Var
                };

                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "const_spec" || child.kind() == "var_spec" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            let name = self.node_text(&name_node, source);

                            symbols.push(Symbol {
                                name,
                                package: module_path.to_string(),
                                package_name: package_name.to_string(),
                                kind,
                                version: None,
                                import_path: module_path.to_string(),
                                doc: None,
                                signature: None,
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        // Recursively process children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_symbols_from_node(&child, source, module_path, package_name, symbols);
        }
    }

    /// Get text content of a node
    fn node_text(&self, node: &Node, source: &[u8]) -> String {
        node.utf8_text(source).unwrap_or("").to_string()
    }

    /// Extract function signature
    fn extract_function_signature(&self, node: &Node, source: &[u8]) -> String {
        let mut parts = Vec::new();

        if let Some(params) = node.child_by_field_name("parameters") {
            parts.push(self.node_text(&params, source));
        }

        if let Some(result) = node.child_by_field_name("result") {
            parts.push(self.node_text(&result, source));
        }

        if parts.is_empty() {
            "func()".to_string()
        } else {
            format!("func{}", parts.join(" "))
        }
    }

    /// Extract receiver type for methods
    fn extract_receiver(&self, node: &Node, source: &[u8]) -> String {
        if let Some(recv) = node.child_by_field_name("receiver") {
            let recv_text = self.node_text(&recv, source);
            // Extract type name from receiver (e.g., "(c *Client)" -> "Client")
            recv_text
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .last()
                .map(|s| s.trim_start_matches('*').to_string())
                .unwrap_or_default()
        } else {
            "Unknown".to_string()
        }
    }

    /// Determine the kind of a type declaration
    fn determine_type_kind(&self, type_spec: &Node) -> SymbolKind {
        if let Some(type_node) = type_spec.child_by_field_name("type") {
            match type_node.kind() {
                "interface_type" => SymbolKind::Interface,
                "struct_type" => SymbolKind::Struct,
                _ => SymbolKind::Type,
            }
        } else {
            SymbolKind::Type
        }
    }

    /// Extract documentation comment (/// or /* */ style)
    fn extract_doc_comment(&self, _node: &Node, _source: &[u8]) -> Option<String> {
        // This is a simplified version - full implementation would look at preceding comments
        None
    }
}

impl Clone for GoModuleParser {
    fn clone(&self) -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_go_mod() {
        let parser = GoModuleParser::new();
        let content = r#"module github.com/example/test

go 1.21

require (
    github.com/some/dep v1.0.0
)
"#;

        let (module_path, go_version) = parser.parse_go_mod_content(content).unwrap();
        assert_eq!(module_path, "github.com/example/test");
        assert_eq!(go_version, "1.21");
    }

    #[test]
    fn test_extract_package_name() {
        let parser = GoModuleParser::new();
        let content = r#"package mypackage

import "fmt"

func main() {}
"#;

        assert_eq!(parser.extract_package_name(content), "mypackage");
    }
}
