use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Priority level for resource directories
/// Following Android's standard resource priority order:
/// Library Dependencies < Additional Resources < Main Resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePriority {
    /// Library dependencies / AAR files (lowest priority)
    /// These are external dependencies and have the lowest priority
    Library(usize),
    /// Additional resource directories (medium priority, ordered by index)
    /// These represent shared/common resources used across flavors
    /// In additionalResourceDirs, earlier entries have lower priority
    Additional(usize),
    /// Main resource directory (highest priority)
    /// This is the main source set (src/main/res)
    Main,
}

impl ResourcePriority {
    /// Get numeric priority for comparison
    /// Lower values mean lower priority (will be overridden)
    /// Following Android standard: Library (0-999) < Additional (1000-1999) < Main (2000)
    pub fn value(&self) -> usize {
        match self {
            ResourcePriority::Library(idx) => *idx,
            ResourcePriority::Additional(idx) => 1000 + idx,
            ResourcePriority::Main => 2000,
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
                self.conflicts
                    .push((normalized.clone(), existing.clone(), info.clone()));
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
        // Correct Android priority order: Library < Additional < Main
        assert!(ResourcePriority::Library(0).value() < ResourcePriority::Additional(0).value());
        assert!(ResourcePriority::Additional(0).value() < ResourcePriority::Main.value());
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

        // Add additional resource with same path (should override library)
        let additional = ResourceInfo {
            source_path: PathBuf::from("/additional/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/additional_drawable_icon.png.flat"),
            resource_dir: PathBuf::from("/additional/res"),
            priority: ResourcePriority::Additional(0),
            normalized_path: "res/drawable/icon.png".to_string(),
        };

        assert!(tracker.add_resource(additional));
        assert_eq!(tracker.stats(), (1, 1)); // Still 1 resource, 1 conflict

        // Add main resource with same path (should override additional)
        let main = ResourceInfo {
            source_path: PathBuf::from("/main/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/main_drawable_icon.png.flat"),
            resource_dir: PathBuf::from("/main/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/drawable/icon.png".to_string(),
        };

        assert!(tracker.add_resource(main));
        assert_eq!(tracker.stats(), (1, 2)); // Still 1 resource, 2 conflicts
    }

    #[test]
    fn test_priority_value_boundaries() {
        // Library(0) = 0, Library(999) = 999
        assert_eq!(ResourcePriority::Library(0).value(), 0);
        assert_eq!(ResourcePriority::Library(999).value(), 999);

        // Additional(0) = 1000, Additional(999) = 1999
        assert_eq!(ResourcePriority::Additional(0).value(), 1000);
        assert_eq!(ResourcePriority::Additional(999).value(), 1999);

        // Main = 2000
        assert_eq!(ResourcePriority::Main.value(), 2000);
    }

    #[test]
    fn test_priority_partial_ord() {
        // Verify derived PartialOrd works with enum ordering
        assert!(ResourcePriority::Library(5) < ResourcePriority::Library(10));
        assert!(ResourcePriority::Library(999) < ResourcePriority::Additional(0));
        assert!(ResourcePriority::Additional(999) < ResourcePriority::Main);
        assert!(ResourcePriority::Main > ResourcePriority::Additional(0));
        assert!(ResourcePriority::Main > ResourcePriority::Library(0));
        // Additional wins over Library with any index
        assert!(ResourcePriority::Additional(0) > ResourcePriority::Library(999));
    }

    #[test]
    fn test_tracker_empty() {
        let tracker = ResourcePriorityTracker::new();
        assert_eq!(tracker.stats(), (0, 0));
        let flat_files = tracker.get_final_flat_files();
        assert!(flat_files.is_empty());
    }

    #[test]
    fn test_add_resource_no_conflict() {
        let mut tracker = ResourcePriorityTracker::new();

        let res1 = ResourceInfo {
            source_path: PathBuf::from("/main/res/layout/main.xml"),
            flat_file: PathBuf::from("/build/layout_main.xml.flat"),
            resource_dir: PathBuf::from("/main/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/layout/main.xml".to_string(),
        };
        assert!(!tracker.add_resource(res1));

        let res2 = ResourceInfo {
            source_path: PathBuf::from("/main/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/drawable_icon.png.flat"),
            resource_dir: PathBuf::from("/main/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/drawable/icon.png".to_string(),
        };
        assert!(!tracker.add_resource(res2));

        assert_eq!(tracker.stats(), (2, 0));
    }

    #[test]
    fn test_same_priority_conflict() {
        let mut tracker = ResourcePriorityTracker::new();

        let res1 = ResourceInfo {
            source_path: PathBuf::from("/a/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/a_drawable_icon.png.flat"),
            resource_dir: PathBuf::from("/a/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/drawable/icon.png".to_string(),
        };
        assert!(!tracker.add_resource(res1));

        // Same priority, same path — should NOT override
        let res2 = ResourceInfo {
            source_path: PathBuf::from("/b/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/b_drawable_icon.png.flat"),
            resource_dir: PathBuf::from("/b/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/drawable/icon.png".to_string(),
        };
        assert!(!tracker.add_resource(res2));

        // Still only 1 resource (first one wins on equal priority).
        assert_eq!(tracker.stats(), (1, 1));
    }

    #[test]
    fn test_lower_priority_loses() {
        let mut tracker = ResourcePriorityTracker::new();

        // Add Main first (highest priority)
        let main = ResourceInfo {
            source_path: PathBuf::from("/main/res/values/colors.xml"),
            flat_file: PathBuf::from("/build/main_values_colors.arsc.flat"),
            resource_dir: PathBuf::from("/main/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/values/colors.xml".to_string(),
        };
        assert!(!tracker.add_resource(main));

        // Lower priority should NOT override
        let lib = ResourceInfo {
            source_path: PathBuf::from("/lib/res/values/colors.xml"),
            flat_file: PathBuf::from("/build/lib_values_colors.arsc.flat"),
            resource_dir: PathBuf::from("/lib/res"),
            priority: ResourcePriority::Library(0),
            normalized_path: "res/values/colors.xml".to_string(),
        };
        assert!(!tracker.add_resource(lib));

        assert_eq!(tracker.stats(), (1, 1));
        // Main should still be the one in the map
        let files = tracker.get_final_flat_files();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0],
            PathBuf::from("/build/main_values_colors.arsc.flat")
        );
    }

    #[test]
    fn test_get_final_flat_files_ordering() {
        let mut tracker = ResourcePriorityTracker::new();

        // Add in reverse priority order
        let main = ResourceInfo {
            source_path: PathBuf::from("/main/res/values/strings.xml"),
            flat_file: PathBuf::from("/build/main_strings.arsc.flat"),
            resource_dir: PathBuf::from("/main/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/values/strings.xml".to_string(),
        };
        tracker.add_resource(main);

        let additional = ResourceInfo {
            source_path: PathBuf::from("/add/res/drawable/icon.png"),
            flat_file: PathBuf::from("/build/add_icon.png.flat"),
            resource_dir: PathBuf::from("/add/res"),
            priority: ResourcePriority::Additional(0),
            normalized_path: "res/drawable/icon.png".to_string(),
        };
        tracker.add_resource(additional);

        let lib = ResourceInfo {
            source_path: PathBuf::from("/lib/res/layout/main.xml"),
            flat_file: PathBuf::from("/build/lib_layout.xml.flat"),
            resource_dir: PathBuf::from("/lib/res"),
            priority: ResourcePriority::Library(0),
            normalized_path: "res/layout/main.xml".to_string(),
        };
        tracker.add_resource(lib);

        // Files should be sorted by priority (lowest first)
        let files = tracker.get_final_flat_files();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0], PathBuf::from("/build/lib_layout.xml.flat"));
        assert_eq!(files[1], PathBuf::from("/build/add_icon.png.flat"));
        assert_eq!(files[2], PathBuf::from("/build/main_strings.arsc.flat"));
    }

    #[test]
    fn test_normalize_resource_path_basic() {
        let resource_file = Path::new("/home/project/src/main/res/drawable/icon.png");
        let resource_dir = Path::new("/home/project/src/main/res");
        let result = normalize_resource_path(resource_file, resource_dir).unwrap();
        assert_eq!(result, "res/drawable/icon.png");
    }

    #[test]
    fn test_normalize_resource_path_values() {
        let resource_file = Path::new("/project/res/values/strings.xml");
        let resource_dir = Path::new("/project/res");
        let result = normalize_resource_path(resource_file, resource_dir).unwrap();
        assert_eq!(result, "res/values/strings.xml");
    }

    #[test]
    fn test_normalize_resource_path_qualified() {
        let resource_file = Path::new("/app/res/values-en/strings.xml");
        let resource_dir = Path::new("/app/res");
        let result = normalize_resource_path(resource_file, resource_dir).unwrap();
        assert_eq!(result, "res/values-en/strings.xml");
    }

    #[test]
    fn test_normalize_resource_path_not_under_dir() {
        let resource_file = Path::new("/other/project/res/icon.png");
        let resource_dir = Path::new("/home/project/res");
        let result = normalize_resource_path(resource_file, resource_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_resource_path_backslash() {
        // Paths with backslashes (Windows-style, already normalized by Rust Path)
        let resource_file = Path::new("/proj/res/drawable-hdpi\\icon.png");
        let resource_dir = Path::new("/proj/res");
        let result = normalize_resource_path(resource_file, resource_dir).unwrap();
        // Backslashes in the relative path get normalized to forward slashes
        assert_eq!(result, "res/drawable-hdpi/icon.png");
    }

    #[test]
    fn test_find_matching_flat_file_layout() {
        let source = Path::new("/proj/res/layout/activity_main.xml");
        let flat_files = vec![
            PathBuf::from("/build/layout_activity_main.xml.flat"),
            PathBuf::from("/build/drawable_icon.png.flat"),
        ];
        let result = find_matching_flat_file(source, &flat_files);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/build/layout_activity_main.xml.flat")
        );
    }

    #[test]
    fn test_find_matching_flat_file_values() {
        let source = Path::new("/proj/res/values/strings.xml");
        let flat_files = vec![
            PathBuf::from("/build/values_strings.arsc.flat"),
            PathBuf::from("/build/drawable_icon.png.flat"),
        ];
        let result = find_matching_flat_file(source, &flat_files);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/build/values_strings.arsc.flat")
        );
    }

    #[test]
    fn test_find_matching_flat_file_not_found() {
        let source = Path::new("/proj/res/layout/missing.xml");
        let flat_files = vec![PathBuf::from("/build/drawable_icon.png.flat")];
        let result = find_matching_flat_file(source, &flat_files);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_matching_flat_file_empty_flat_files() {
        let source = Path::new("/proj/res/drawable/icon.png");
        let flat_files: Vec<PathBuf> = vec![];
        let result = find_matching_flat_file(source, &flat_files);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_resources_with_priority_empty_dir() {
        let non_existent = Path::new("/non/existent/dir");
        let flat_files: Vec<PathBuf> = vec![];
        let result =
            find_resources_with_priority(non_existent, &flat_files, ResourcePriority::Main)
                .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_conflict_logging_empty() {
        let tracker = ResourcePriorityTracker::new();
        // log_conflicts should not panic on empty conflicts
        tracker.log_conflicts();
    }

    #[test]
    fn test_conflict_logging_with_conflicts() {
        let mut tracker = ResourcePriorityTracker::new();

        let lib = ResourceInfo {
            source_path: PathBuf::from("/lib/res/icon.png"),
            flat_file: PathBuf::from("/build/lib_icon.flat"),
            resource_dir: PathBuf::from("/lib/res"),
            priority: ResourcePriority::Library(0),
            normalized_path: "res/icon.png".to_string(),
        };
        tracker.add_resource(lib);

        let main = ResourceInfo {
            source_path: PathBuf::from("/main/res/icon.png"),
            flat_file: PathBuf::from("/build/main_icon.flat"),
            resource_dir: PathBuf::from("/main/res"),
            priority: ResourcePriority::Main,
            normalized_path: "res/icon.png".to_string(),
        };
        tracker.add_resource(main);

        // Should not panic
        tracker.log_conflicts();
        assert_eq!(tracker.stats(), (1, 1));
    }

    #[test]
    fn test_priority_derive_traits() {
        // Test Clone
        let p1 = ResourcePriority::Library(5);
        let p2 = p1;
        assert_eq!(p1, p2);

        // Test Copy
        let p3 = p1;
        assert_eq!(p1, p3);

        // Test Debug (compile-time check)
        let _ = format!("{:?}", ResourcePriority::Main);

        // Test Eq + PartialEq
        assert_eq!(ResourcePriority::Library(5), ResourcePriority::Library(5));
        assert_ne!(ResourcePriority::Library(5), ResourcePriority::Library(6));
        assert_ne!(
            ResourcePriority::Library(0),
            ResourcePriority::Additional(0)
        );
        assert_ne!(ResourcePriority::Additional(0), ResourcePriority::Main);
    }
}
