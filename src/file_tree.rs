//! High-performance in-process File Explorer and Tree model generator.
//! Scans local directories and outputs hierarchical tree node data with sub-millisecond latency.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileNode {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    pub icon: String,
    pub level: i32,
    pub is_folder: bool,
    pub expanded: bool,
    pub has_children: bool,
    pub badge: String,
}

pub struct FileTreeManager {
    root_path: PathBuf,
    expanded_paths: HashSet<String>,
}

impl FileTreeManager {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        let mut expanded = HashSet::new();
        // Expand root by default
        expanded.insert("root".to_string());
        expanded.insert("src".to_string());
        expanded.insert("ui".to_string());

        Self {
            root_path: root_path.as_ref().to_path_buf(),
            expanded_paths: expanded,
        }
    }

    /// Toggles the expanded state of a folder node by ID/path
    pub fn toggle_expand(&mut self, node_id: &str) -> bool {
        if self.expanded_paths.contains(node_id) {
            self.expanded_paths.remove(node_id);
            false
        } else {
            self.expanded_paths.insert(node_id.to_string());
            true
        }
    }

    /// Checks if a node ID is expanded
    pub fn is_expanded(&self, node_id: &str) -> bool {
        self.expanded_paths.contains(node_id)
    }

    /// Scans the directory tree and generates a flattened visible node list
    pub fn generate_visible_nodes(&self) -> Vec<FileNode> {
        let mut nodes = Vec::new();
        
        let root_label = self
            .root_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Project Root".to_string());

        let root_id = "root".to_string();
        let is_root_expanded = self.expanded_paths.contains(&root_id);

        nodes.push(FileNode {
            id: root_id.clone(),
            label: root_label,
            path: self.root_path.clone(),
            icon: "folder".to_string(),
            level: 0,
            is_folder: true,
            expanded: is_root_expanded,
            has_children: true,
            badge: "root".to_string(),
        });

        if is_root_expanded {
            self.scan_recursive(&self.root_path, 1, &mut nodes);
        }

        nodes
    }

    fn scan_recursive(&self, dir: &Path, level: i32, out: &mut Vec<FileNode>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files, target caches, and git
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }

            if path.is_dir() {
                dirs.push((name, path));
            } else {
                files.push((name, path));
            }
        }

        // Sort directories and files alphabetically
        dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

        // Process directories
        for (name, path) in dirs {
            let id = path.to_string_lossy().to_string();
            let has_children = fs::read_dir(&path)
                .map(|mut r| r.next().is_some())
                .unwrap_or(false);
            
            let is_expanded = self.expanded_paths.contains(&id) 
                || self.expanded_paths.contains(&name);

            out.push(FileNode {
                id: id.clone(),
                label: name,
                path: path.clone(),
                icon: "folder".to_string(),
                level,
                is_folder: true,
                expanded: is_expanded,
                has_children,
                badge: "dir".to_string(),
            });

            if is_expanded {
                self.scan_recursive(&path, level + 1, out);
            }
        }

        // Process files
        for (name, path) in files {
            let id = path.to_string_lossy().to_string();
            let icon = detect_file_icon(&name);
            let badge = format_file_size(&path);

            out.push(FileNode {
                id,
                label: name,
                path,
                icon,
                level,
                is_folder: false,
                expanded: false,
                has_children: false,
                badge,
            });
        }
    }
}

fn detect_file_icon(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.ends_with(".rs") 
        || lower.ends_with(".slint") 
        || lower.ends_with(".toml") 
        || lower.ends_with(".json") 
        || lower.ends_with(".md")
        || lower.ends_with(".lock")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
    {
        "code".to_string()
    } else {
        "file".to_string()
    }
}

fn format_file_size(path: &Path) -> String {
    if let Ok(metadata) = fs::metadata(path) {
        let bytes = metadata.len();
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    } else {
        "".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tree_scanning() {
        let manager = FileTreeManager::new(".");
        let nodes = manager.generate_visible_nodes();
        assert!(!nodes.is_empty());
        assert_eq!(nodes[0].level, 0);
        assert!(nodes[0].is_folder);
        // Verify root contains items
        assert!(nodes.len() > 1);
    }

    #[test]
    fn test_file_tree_expand_collapse() {
        let mut manager = FileTreeManager::new(".");
        let initial_count = manager.generate_visible_nodes().len();

        // Collapse root
        manager.toggle_expand("root");
        let collapsed_count = manager.generate_visible_nodes().len();
        assert_eq!(collapsed_count, 1);

        // Re-expand root
        manager.toggle_expand("root");
        let reexpanded_count = manager.generate_visible_nodes().len();
        assert_eq!(reexpanded_count, initial_count);
    }
}

