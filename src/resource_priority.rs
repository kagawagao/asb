use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Priority level for resource directories
/// Following Android's standard resource priority order:
/// Library Dependencies < Main Resources < Product Flavor < Build Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePriority {
    /// Library dependencies / AAR files (lowest priority)
    /// These are external dependencies and have the lowest priority
    Library(usize),
    /// Main resource directory (medium priority)
    /// This is the main source set (src/main/res)
    Main,
    /// Additional resource directories (highest priority, ordered by index)
    /// These represent Product Flavors and Build Types
    /// In additionalResourceDirs, earlier entries are flavors, later entries are build types
    Additional(usize),
}

impl ResourcePriority {
    /// Get numeric priority for comparison
    /// Lower values mean lower priority (will be overridden)
    /// Following Android standard: Library (0-999) < Main (1000) < Additional (2000+)
    pub fn value(&self) -> usize {
        match self {
            ResourcePriority::Library(idx) => *idx,
            ResourcePriority::Main => 1000,
            ResourcePriority::Additional(idx) => 2000 + idx,
        }
    }
}

/// Metadata for a compiled resource
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResourceInfo {
    /// Original source file path
    pub source_path: PathBuf,
    /// Compiled flat file path
    pub flat_file: PathBuf,
    /// Resource directory this came from
    pub resource_dir: PathBuf,
    /// Priority level
    pub priority: ResourcePriority,
    /// Normalized resource path (for conflict detection)
    /// e.g., "res/drawable/icon.png" or "res/values/strings.xml"
    pub normalized_path: String,
}

/// Tracks resources and their priorities for conflict resolution
#[allow(dead_code)]
pub struct ResourcePriorityTracker {
    /// Map from normalized resource path to resource info
    resources: HashMap<String, ResourceInfo>,
    /// Track conflicts for logging
    conflicts: Vec<(String, ResourceInfo, ResourceInfo)>,
}

impl ResourcePriorityTracker {
    /// Create a new tracker
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Add a resource with its priority
    /// Returns true if this resource overrides a previous one
    #[allow(dead_code)]
    pub fn add_resource(&mut self, info: ResourceInfo) -> bool {
        let normalized = info.normalized_path.clone();

        if let Some(existing) = self.resources.get(&normalized) {
            // Check if new resource has higher priority
            if info.priority.value() > existing.priority.value() {
                // New resource wins - record the conflict
                debug!(
                    "Resource override: {} (priority {:?}) overrides {} (priority {:?})",
                    info.source_path.display(),
                    info.priority,
                    existing.source_path.display(),
                    existing.priority
                );

                self.conflicts
                    .push((normalized.clone(), existing.clone(), info.clone()));

                self.resources.insert(normalized, info);
                return true;
            } else if info.priority.value() < existing.priority.value() {
                // Existing resource wins - still record for logging
                debug!(
                    "Resource ignored: {} (priority {:?}) loses to {} (priority {:?})",
                    info.source_path.display(),
                    info.priority,
                    existing.source_path.display(),
                    existing.priority
                );

                self.conflicts
                    .push((normalized.clone(), info.clone(), existing.clone()));

                return false;
            } else {
                // Same priority - this shouldn't happen with proper indexing
                warn!(
                    "Resource conflict with same priority: {} and {}",
                    info.source_path.display(),
                    existing.source_path.display()
                );
                return false;
            }
        }

        // No conflict - add the resource
        self.resources.insert(normalized, info);
        false
    }

    /// Get the final list of flat files to use, in priority order
    #[allow(dead_code)]
    pub fn get_final_flat_files(&self) -> Vec<PathBuf> {
        // Collect all resources
        let mut resources: Vec<&ResourceInfo> = self.resources.values().collect();

        // Sort by priority (lower priority first, higher priority last)
        // This ensures that when aapt2 processes them with --auto-add-overlay,
        // higher priority resources override lower priority ones
        resources.sort_by_key(|r| r.priority.value());

        // Extract flat file paths
        resources.iter().map(|r| r.flat_file.clone()).collect()
    }

    /// Log conflicts to help users understand resource overrides
    #[allow(dead_code)]
    pub fn log_conflicts(&self) {
        if self.conflicts.is_empty() {
            debug!("No resource conflicts detected");
            return;
        }

        info!(
            "Resource conflicts resolved: {} overrides detected",
            self.conflicts.len()
        );

        for (path, lower, higher) in &self.conflicts {
            info!(
                "  {} overridden by {} (from {:?} to {:?})",
                path,
                higher.source_path.display(),
                lower.priority,
                higher.priority
            );
        }
    }

    /// Get statistics about resources
    #[allow(dead_code)]
    pub fn stats(&self) -> (usize, usize) {
        (self.resources.len(), self.conflicts.len())
    }
}

/// Normalize a resource path for comparison
/// Removes the base resource directory and standardizes the path format
/// e.g., "/path/to/res/drawable-hdpi/icon.png" -> "res/drawable-hdpi/icon.png"
#[allow(dead_code)]
pub fn normalize_resource_path(resource_file: &Path, resource_dir: &Path) -> Result<String> {
    // Get the relative path from resource directory
    let rel_path = resource_file
        .strip_prefix(resource_dir)
        .map_err(|_| anyhow::anyhow!("Resource file not under resource directory"))?;

    // Construct normalized path with "res/" prefix
    let normalized = format!("res/{}", rel_path.display());

    // Normalize path separators to forward slashes for consistency
    let normalized = normalized.replace('\\', "/");

    Ok(normalized)
}

/// Find all resource files in a directory and create ResourceInfo entries
#[allow(dead_code)]
pub fn find_resources_with_priority(
    resource_dir: &Path,
    compiled_flat_files: &[PathBuf],
    priority: ResourcePriority,
) -> Result<Vec<ResourceInfo>> {
    let mut resources = Vec::new();

    if !resource_dir.exists() {
        return Ok(resources);
    }

    // Walk through all files in the resource directory
    for entry in WalkDir::new(resource_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let source_path = entry.path();

        // Skip hidden files and system files
        if let Some(name) = source_path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name == "Thumbs.db" {
                continue;
            }
        }

        // Normalize the resource path for comparison
        let normalized = normalize_resource_path(source_path, resource_dir)?;

        // Find the corresponding flat file
        // This is a heuristic match based on file naming conventions
        if let Some(flat_file) = find_matching_flat_file(source_path, compiled_flat_files) {
            resources.push(ResourceInfo {
                source_path: source_path.to_path_buf(),
                flat_file,
                resource_dir: resource_dir.to_path_buf(),
                priority,
                normalized_path: normalized,
            });
        }
    }

    Ok(resources)
}

/// Find the flat file that corresponds to a source resource file
/// aapt2 generates flat files with specific naming patterns:
/// - layout_activity_main.xml.flat for res/layout/activity_main.xml
/// - drawable_icon.xml.flat for res/drawable/icon.xml
/// - values_strings.arsc.flat for res/values/strings.xml
#[allow(dead_code)]
fn find_matching_flat_file(source_path: &Path, flat_files: &[PathBuf]) -> Option<PathBuf> {
    // Get the resource type directory (e.g., "drawable", "layout", "values-en")
    let parent = source_path.parent()?;
    let parent_name = parent.file_name()?.to_str()?;
    let file_name = source_path.file_name()?.to_str()?;

    // Generate possible flat file names based on aapt2 naming conventions
    let possible_names = if parent_name.starts_with("values") {
        // For values resources: values_strings.arsc.flat or values-en_strings.arsc.flat
        let stem = source_path.file_stem()?.to_str()?;
        vec![format!("{}_{}.arsc.flat", parent_name, stem)]
    } else {
        // For other resources: layout_activity_main.xml.flat
        vec![format!("{}_{}.flat", parent_name, file_name)]
    };

    // Search for matching flat file - combine exact and pattern matching in single pass
    for flat_file in flat_files {
        if let Some(flat_name) = flat_file.file_name().and_then(|n| n.to_str()) {
            // Try exact match first
            for possible in &possible_names {
                if flat_name == possible {
                    return Some(flat_file.clone());
                }
            }

            // Try pattern matching if exact match fails
            // Check if the flat file name starts with the resource type
            if flat_name.starts_with(parent_name) {
                // And contains the file name (without extension for values)
                if parent_name.starts_with("values") {
                    if let Some(stem) = source_path.file_stem().and_then(|s| s.to_str()) {
                        if flat_name.contains(stem) {
                            return Some(flat_file.clone());
                        }
                    }
                } else {
                    if flat_name.contains(file_name) {
                        return Some(flat_file.clone());
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        // Correct Android priority order: Library < Main < Additional
        assert!(ResourcePriority::Library(0).value() < ResourcePriority::Main.value());
        assert!(ResourcePriority::Main.value() < ResourcePriority::Additional(0).value());
        assert!(ResourcePriority::Library(0).value() < ResourcePriority::Library(1).value());
        assert!(ResourcePriority::Additional(0).value() < ResourcePriority::Additional(1).value());
    }

    #[test]
    fn test_resource_override() {
        let mut tracker = ResourcePriorityTracker::new();

        // Add library resource (lowest priority)
        let library = ResourceInfo {
            source_path: PathBuf::from("/library/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/lib_drawable_icon.png.flat"),
            resource_dir: PathBuf::from("/library/res"),
            priority: ResourcePriority::Library(0),
            normalized_path: "res/drawable/icon.png".to_string(),
        };

        assert!(!tracker.add_resource(library));
        assert_eq!(tracker.stats(), (1, 0));

        // Add main resource with same path (should override library)
        let main = ResourceInfo {
            source_path: PathBuf::from("/main/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/main_drawable_icon.png.flat"),
            resource_dir: PathBuf::from("/main/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/drawable/icon.png".to_string(),
        };

        assert!(tracker.add_resource(main));
        assert_eq!(tracker.stats(), (1, 1)); // Still 1 resource, 1 conflict

        // Add additional resource with same path (should override main)
        let additional = ResourceInfo {
            source_path: PathBuf::from("/additional/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/additional_drawable_icon.png.flat"),
            resource_dir: PathBuf::from("/additional/res"),
            priority: ResourcePriority::Additional(0),
            normalized_path: "res/drawable/icon.png".to_string(),
        };

        assert!(tracker.add_resource(additional));
        assert_eq!(tracker.stats(), (1, 2)); // Still 1 resource, 2 conflicts
    }
}
