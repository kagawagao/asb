use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use anyhow::Result;
use tracing::info;

use crate::types::BuildConfig;

/// Represents a build configuration with its index for ordering
#[derive(Debug, Clone)]
pub struct ConfigWithIndex {
    pub index: usize,
    pub config: BuildConfig,
}

/// Represents common dependencies shared across multiple configurations
#[derive(Debug, Clone)]
pub struct CommonDependency {
    /// The resource directory path
    pub resource_dir: PathBuf,
    /// Indices of configurations that depend on this resource directory
    pub dependent_configs: Vec<usize>,
}

/// Group configurations by their dependencies based on shared resource directories
/// 
/// Analyzes the `additionalResourceDirs` field to detect dependencies between configurations.
/// A configuration depends on another if it references a resource directory that is the main
/// resource directory of another configuration.
/// 
/// # Returns
/// 
/// A tuple of:
/// - `independent_configs`: Configurations with no dependencies that can be built in parallel
/// - `dependency_groups`: Groups of dependent configurations that must be built sequentially
///   within each group (in topological order), though different groups can be processed in parallel
pub fn group_configs_by_dependencies(configs: Vec<BuildConfig>) -> Result<(Vec<ConfigWithIndex>, Vec<Vec<ConfigWithIndex>>)> {
    if configs.is_empty() {
        return Ok((vec![], vec![]));
    }
    
    if configs.len() == 1 {
        return Ok((vec![ConfigWithIndex { index: 0, config: configs.into_iter().next().unwrap() }], vec![]));
    }

    // Build a map of resource directories to config indices that use them
    let mut resource_dir_to_configs: HashMap<String, HashSet<usize>> = HashMap::new();
    
    for (idx, config) in configs.iter().enumerate() {
        // Normalize and register the main resource directory
        let main_res = normalize_path(&config.resource_dir);
        resource_dir_to_configs.entry(main_res).or_insert_with(HashSet::new).insert(idx);
        
        // Register additional resource directories if present
        if let Some(additional_dirs) = &config.additional_resource_dirs {
            for dir in additional_dirs {
                let normalized = normalize_path(dir);
                resource_dir_to_configs.entry(normalized).or_insert_with(HashSet::new).insert(idx);
            }
        }
    }
    
    // Build dependency graph: config_idx -> Vec<config_idx it depends on>
    let mut dependencies: HashMap<usize, Vec<usize>> = HashMap::new();
    
    for (idx, config) in configs.iter().enumerate() {
        let mut deps = Vec::new();
        
        // Check if any of this config's additional resource dirs are provided by other configs
        if let Some(additional_dirs) = &config.additional_resource_dirs {
            for dir in additional_dirs {
                let normalized = normalize_path(dir);
                
                // Find which configs provide this resource directory
                if let Some(providers) = resource_dir_to_configs.get(&normalized) {
                    for &provider_idx in providers {
                        // A config depends on another if the other provides a resource dir it needs
                        // and it's not itself
                        if provider_idx != idx {
                            // Check if provider_idx's main resource_dir matches this additional dir
                            let provider_main = normalize_path(&configs[provider_idx].resource_dir);
                            if provider_main == normalized {
                                deps.push(provider_idx);
                            }
                        }
                    }
                }
            }
        }
        
        if !deps.is_empty() {
            dependencies.insert(idx, deps);
        }
    }
    
    // Perform topological sort to determine build order
    let sorted_indices = topological_sort(configs.len(), &dependencies)?;
    
    // Separate into independent and dependent groups
    let mut independent = Vec::new();
    let mut dependent_groups: Vec<Vec<ConfigWithIndex>> = Vec::new();
    let mut current_group: Vec<ConfigWithIndex> = Vec::new();
    let mut in_dependency_chain = HashSet::new();
    
    // Mark all configs that are part of dependency chains
    for (&config_idx, deps) in &dependencies {
        in_dependency_chain.insert(config_idx);
        for &dep in deps {
            in_dependency_chain.insert(dep);
        }
    }
    
    // Process sorted indices
    for idx in sorted_indices {
        let config = configs[idx].clone();
        let config_with_idx = ConfigWithIndex { index: idx, config };
        
        if in_dependency_chain.contains(&idx) {
            current_group.push(config_with_idx);
        } else {
            independent.push(config_with_idx);
        }
    }
    
    if !current_group.is_empty() {
        dependent_groups.push(current_group);
    }
    
    Ok((independent, dependent_groups))
}

/// Normalize a path to a string for comparison purposes
/// 
/// Attempts to convert the path to an absolute canonical path to ensure that different
/// representations of the same path (e.g., relative vs absolute, with/without trailing slashes)
/// are treated as equal. If canonicalization fails (e.g., path doesn't exist yet), falls back
/// to normalizing the string representation by replacing backslashes with forward slashes.
/// 
/// # Arguments
/// 
/// * `path` - The path to normalize
/// 
/// # Returns
/// 
/// A normalized string representation of the path suitable for comparison
fn normalize_path(path: &PathBuf) -> String {
    // Convert to absolute path if possible, otherwise use as-is
    if let Ok(abs_path) = std::fs::canonicalize(path) {
        abs_path.to_string_lossy().to_string()
    } else {
        // If path doesn't exist yet, just normalize the string representation
        path.to_string_lossy().replace('\\', "/")
    }
}

/// Perform topological sort on the dependency graph using Kahn's algorithm
/// 
/// Topological sorting arranges configurations so that dependencies are always built before
/// their dependents. Uses a breadth-first approach with a queue (VecDeque) for deterministic
/// ordering. Detects circular dependencies and returns an error if found.
/// 
/// # Arguments
/// 
/// * `num_configs` - Total number of configurations to sort
/// * `dependencies` - A map where keys are dependent config indices and values are vectors of
///   the config indices they depend on (i.e., `dependent -> [dependencies]`)
/// 
/// # Returns
/// 
/// A vector of configuration indices in topological order (dependencies before dependents),
/// or an error if a circular dependency is detected
fn topological_sort(num_configs: usize, dependencies: &HashMap<usize, Vec<usize>>) -> Result<Vec<usize>> {
    let mut in_degree = vec![0; num_configs];
    let mut adj_list: HashMap<usize, Vec<usize>> = HashMap::new();
    
    // Build adjacency list and calculate in-degrees
    // dependencies maps: dependent -> dependencies
    // We need: dependency -> dependents for topological sort
    for (dependent, deps) in dependencies {
        for &dependency in deps {
            adj_list.entry(dependency).or_insert_with(Vec::new).push(*dependent);
            in_degree[*dependent] += 1;
        }
    }
    
    // Find all nodes with in-degree 0 (no dependencies)
    let mut queue: std::collections::VecDeque<usize> = (0..num_configs)
        .filter(|&i| in_degree[i] == 0)
        .collect();
    
    let mut sorted = Vec::new();
    
    while let Some(node) = queue.pop_front() {
        sorted.push(node);
        
        // Reduce in-degree for all dependents
        if let Some(dependents) = adj_list.get(&node) {
            for &dependent in dependents {
                in_degree[dependent] -= 1;
                if in_degree[dependent] == 0 {
                    queue.push_back(dependent);
                }
            }
        }
    }
    
    // Check for cycles
    if sorted.len() != num_configs {
        anyhow::bail!("Circular dependency detected in configuration dependencies");
    }
    
    Ok(sorted)
}

/// Extract common dependencies from multiple configurations
/// 
/// Analyzes all configurations to identify resource directories that are used by multiple apps
/// as additional resource directories. These are considered common dependencies that should be
/// compiled separately and cached for reuse.
/// 
/// # Arguments
/// 
/// * `configs` - The list of build configurations to analyze
/// 
/// # Returns
/// 
/// A vector of CommonDependency structs, each representing a resource directory that is
/// shared by multiple configurations. Returns empty vector for 0 or 1 configs since common
/// dependencies require at least 2 apps sharing a resource directory.
pub fn extract_common_dependencies(configs: &[BuildConfig]) -> Vec<CommonDependency> {
    // Early return for trivial cases - need at least 2 configs to have common dependencies
    if configs.len() <= 1 {
        return vec![];
    }
    
    // Map of normalized resource directory paths to:
    // - the configurations that reference them
    // - the original PathBuf (to avoid losing the original path)
    let mut resource_usage: HashMap<String, (Vec<usize>, PathBuf)> = HashMap::new();
    
    // Track which resource directories are main resource dirs
    let mut main_resource_dirs: HashSet<String> = HashSet::new();
    
    // First pass: collect main resource directories
    for config in configs.iter() {
        let main_res = normalize_path(&config.resource_dir);
        main_resource_dirs.insert(main_res.clone());
    }
    
    // Second pass: collect all additional resource directory usage
    for (idx, config) in configs.iter().enumerate() {
        if let Some(additional_dirs) = &config.additional_resource_dirs {
            for dir in additional_dirs {
                let normalized = normalize_path(dir);
                // Track all additional resource dirs, not just those that are main dirs
                resource_usage.entry(normalized)
                    .or_insert_with(|| (Vec::new(), dir.clone()))
                    .0
                    .push(idx);
            }
        }
    }
    
    // Extract common dependencies (resource dirs used by multiple configs)
    let mut common_deps = Vec::new();
    
    for (resource_path, (dependent_indices, original_path)) in resource_usage {
        if dependent_indices.len() > 1 {
            // This is a common dependency used by multiple configurations
            // Prefer using the main resource dir PathBuf if available, otherwise use from additional dirs
            let path_buf = configs.iter()
                .find(|c| normalize_path(&c.resource_dir) == resource_path)
                .map(|c| c.resource_dir.clone())
                .unwrap_or(original_path);
            
            info!(
                "Found common dependency: {} (used by {} configs)",
                path_buf.display(),
                dependent_indices.len()
            );
            common_deps.push(CommonDependency {
                resource_dir: path_buf,
                dependent_configs: dependent_indices,
            });
        }
    }
    
    common_deps
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test config with minimal required fields
    fn test_config(
        resource_dir: &str,
        package_name: &str,
        additional_resource_dirs: Option<Vec<PathBuf>>,
    ) -> BuildConfig {
        BuildConfig {
            resource_dir: PathBuf::from(resource_dir),
            manifest_path: PathBuf::from("./AndroidManifest.xml"),
            output_file: None,
            output_dir: PathBuf::from("./build"),
            package_name: package_name.to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        }
    }

    #[test]
    fn test_single_config() {
        let configs = vec![BuildConfig::default_config()];
        let (independent, dependent) = group_configs_by_dependencies(configs).unwrap();
        assert_eq!(independent.len(), 1);
        assert_eq!(dependent.len(), 0);
    }

    #[test]
    fn test_independent_configs() {
        let config1 = test_config("./res1", "com.example.app1", None);
        let config2 = test_config("./res2", "com.example.app2", None);
        
        let configs = vec![config1, config2];
        let (independent, dependent) = group_configs_by_dependencies(configs).unwrap();
        
        // Both should be independent as they don't share resources
        assert_eq!(independent.len(), 2);
        assert_eq!(dependent.len(), 0);
    }

    #[test]
    fn test_dependent_configs() {
        // Base config
        let base_config = test_config("./base/res", "com.example.base", None);

        // Feature config that depends on base
        let feature_config = test_config(
            "./feature/res",
            "com.example.feature",
            Some(vec![PathBuf::from("./base/res")]),
        );

        let configs = vec![base_config, feature_config];
        let (independent, dependent) = group_configs_by_dependencies(configs).unwrap();

        // Should have dependency group
        assert_eq!(independent.len(), 0);
        assert_eq!(dependent.len(), 1);
        assert_eq!(dependent[0].len(), 2);

        // Base should come before feature in the sorted order
        let sorted_indices: Vec<usize> = dependent[0].iter().map(|c| c.index).collect();
        let base_idx = sorted_indices.iter().position(|&i| i == 0).unwrap();
        let feature_idx = sorted_indices.iter().position(|&i| i == 1).unwrap();
        assert!(base_idx < feature_idx, "Base should be built before feature");
    }

    #[test]
    fn test_multiple_features_depending_on_base() {
        // Base config
        let base_config = test_config("./base/res", "com.example.base", None);

        // Feature1 depends on base
        let feature1_config = test_config(
            "./feature1/res",
            "com.example.feature1",
            Some(vec![PathBuf::from("./base/res")]),
        );

        // Feature2 also depends on base
        let feature2_config = test_config(
            "./feature2/res",
            "com.example.feature2",
            Some(vec![PathBuf::from("./base/res")]),
        );

        let configs = vec![base_config, feature1_config, feature2_config];
        let (independent, dependent) = group_configs_by_dependencies(configs).unwrap();

        // All should be in dependency group
        assert_eq!(independent.len(), 0);
        assert_eq!(dependent.len(), 1);
        assert_eq!(dependent[0].len(), 3);

        // Base should be first
        let sorted_indices: Vec<usize> = dependent[0].iter().map(|c| c.index).collect();
        assert_eq!(sorted_indices[0], 0, "Base should be built first");
    }

    #[test]
    fn test_mixed_independent_and_dependent_configs() {
        // Independent config
        let independent_config = test_config("./independent/res", "com.example.independent", None);

        // Base config
        let base_config = test_config("./base/res", "com.example.base", None);

        // Feature depends on base
        let feature_config = test_config(
            "./feature/res",
            "com.example.feature",
            Some(vec![PathBuf::from("./base/res")]),
        );

        let configs = vec![independent_config, base_config, feature_config];
        let (independent, dependent) = group_configs_by_dependencies(configs).unwrap();

        // Should have 1 independent and 1 dependency group with 2 configs
        assert_eq!(independent.len(), 1);
        assert_eq!(dependent.len(), 1);
        assert_eq!(dependent[0].len(), 2);

        // Verify independent config
        assert_eq!(independent[0].index, 0);
        assert_eq!(independent[0].config.package_name, "com.example.independent");
    }

    #[test]
    fn test_extract_common_dependencies_none() {
        // Single config should have no common dependencies
        let config = BuildConfig::default_config();
        let common_deps = extract_common_dependencies(&[config]);
        assert_eq!(common_deps.len(), 0);
    }

    #[test]
    fn test_extract_common_dependencies_single_shared() {
        // Base config
        let base_config = test_config("./base/res", "com.example.base", None);

        // Feature1 depends on base
        let feature1_config = test_config(
            "./feature1/res",
            "com.example.feature1",
            Some(vec![PathBuf::from("./base/res")]),
        );

        // Feature2 also depends on base
        let feature2_config = test_config(
            "./feature2/res",
            "com.example.feature2",
            Some(vec![PathBuf::from("./base/res")]),
        );

        let configs = vec![base_config, feature1_config, feature2_config];
        let common_deps = extract_common_dependencies(&configs);

        // Should find base/res as a common dependency
        assert_eq!(common_deps.len(), 1);
        assert_eq!(common_deps[0].resource_dir, PathBuf::from("./base/res"));
        assert_eq!(common_deps[0].dependent_configs.len(), 2);
        assert!(common_deps[0].dependent_configs.contains(&1));
        assert!(common_deps[0].dependent_configs.contains(&2));
    }

    #[test]
    fn test_extract_common_dependencies_multiple_shared() {
        // Core config
        let core_config = test_config("./core/res", "com.example.core", None);

        // Shared config
        let shared_config = test_config("./shared/res", "com.example.shared", None);

        // App1 depends on both core and shared
        let app1_config = test_config(
            "./app1/res",
            "com.example.app1",
            Some(vec![
                PathBuf::from("./core/res"),
                PathBuf::from("./shared/res"),
            ]),
        );

        // App2 depends on both core and shared
        let app2_config = test_config(
            "./app2/res",
            "com.example.app2",
            Some(vec![
                PathBuf::from("./core/res"),
                PathBuf::from("./shared/res"),
            ]),
        );

        let configs = vec![core_config, shared_config, app1_config, app2_config];
        let common_deps = extract_common_dependencies(&configs);

        // Should find both core/res and shared/res as common dependencies
        assert_eq!(common_deps.len(), 2);
        
        // Check that both are found (order may vary)
        let core_dep = common_deps.iter().find(|d| d.resource_dir == PathBuf::from("./core/res"));
        let shared_dep = common_deps.iter().find(|d| d.resource_dir == PathBuf::from("./shared/res"));
        
        assert!(core_dep.is_some());
        assert!(shared_dep.is_some());
        
        // Both should have 2 dependents (app1 and app2)
        assert_eq!(core_dep.unwrap().dependent_configs.len(), 2);
        assert_eq!(shared_dep.unwrap().dependent_configs.len(), 2);
    }

    #[test]
    fn test_extract_common_dependencies_from_flavors() {
        use crate::types::{AppConfig, FlavorConfig, MultiAppConfig};
        
        // Base config
        let base_app = AppConfig {
            base_dir: None,
            resource_dir: Some(PathBuf::from("./base/res")),
            manifest_path: Some(PathBuf::from("./base/AndroidManifest.xml")),
            package_name: "com.example.base".to_string(),
            additional_resource_dirs: None,
            output_dir: None,
            output_file: None,
            version_code: None,
            version_name: None,
            flavors: None,
            package_id: None,
        };

        // App with flavors that both depend on base
        let app_with_flavors = AppConfig {
            base_dir: None,
            resource_dir: Some(PathBuf::from("./app/res")),
            manifest_path: Some(PathBuf::from("./app/AndroidManifest.xml")),
            package_name: "com.example.app".to_string(),
            additional_resource_dirs: None,
            output_dir: None,
            output_file: None,
            version_code: None,
            version_name: None,
            flavors: Some(vec![
                FlavorConfig {
                    name: "flavor1".to_string(),
                    base_dir: None,
                    resource_dir: None,
                    manifest_path: None,
                    package_name: None,
                    additional_resource_dirs: Some(vec![PathBuf::from("./base/res")]),
                    output_dir: None,
                    output_file: None,
                    version_code: None,
                    version_name: None,
                    package_id: None,
                },
                FlavorConfig {
                    name: "flavor2".to_string(),
                    base_dir: None,
                    resource_dir: None,
                    manifest_path: None,
                    package_name: None,
                    additional_resource_dirs: Some(vec![PathBuf::from("./base/res")]),
                    output_dir: None,
                    output_file: None,
                    version_code: None,
                    version_name: None,
                    package_id: None,
                },
            ]),
            package_id: None,
        };

        let multi_config = MultiAppConfig {
            base_dir: None,
            output_dir: PathBuf::from("./build"),
            output_file: None,
            android_jar: PathBuf::from("android.jar"),
            aapt2_path: None,
            aar_files: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            stable_ids_file: None,
            max_parallel_builds: None,
            package_id: None,
            apps: vec![base_app, app_with_flavors],
        };

        // Convert to BuildConfigs
        let configs = multi_config.into_build_configs();
        
        // Should have 3 configs: 1 base + 2 flavors
        assert_eq!(configs.len(), 3);
        
        // Extract common dependencies
        let common_deps = extract_common_dependencies(&configs);

        // Should find base/res as a common dependency (used by both flavors)
        assert_eq!(common_deps.len(), 1);
        assert_eq!(common_deps[0].resource_dir, PathBuf::from("./base/res"));
        assert_eq!(common_deps[0].dependent_configs.len(), 2);
        
        // The two flavors should be indices 1 and 2 (base is index 0)
        assert!(common_deps[0].dependent_configs.contains(&1));
        assert!(common_deps[0].dependent_configs.contains(&2));
    }

    #[test]
    fn test_extract_common_dependencies_across_app_flavors() {
        use crate::types::{AppConfig, FlavorConfig, MultiAppConfig};
        
        // Create config matching the example from the comment:
        // Two apps (a and b), each with night and day flavors
        // Both night flavors share ./night/src/main/res
        // Both day flavors share ./day/src/main/res
        
        let app_a = AppConfig {
            base_dir: Some(PathBuf::from("./a/src/main")),
            resource_dir: None,
            manifest_path: None,
            package_name: "com.a".to_string(),
            additional_resource_dirs: None,
            output_dir: None,
            output_file: None,
            version_code: None,
            version_name: None,
            flavors: Some(vec![
                FlavorConfig {
                    name: "night".to_string(),
                    base_dir: None,
                    resource_dir: Some(PathBuf::from("./a/src/main/res-night")),
                    manifest_path: None,
                    package_name: Some("com.a.night".to_string()),
                    additional_resource_dirs: Some(vec![PathBuf::from("./night/src/main/res")]),
                    output_dir: None,
                    output_file: None,
                    version_code: None,
                    version_name: None,
                    package_id: None,
                },
                FlavorConfig {
                    name: "day".to_string(),
                    base_dir: None,
                    resource_dir: Some(PathBuf::from("./a/src/main/res-day")),
                    manifest_path: None,
                    package_name: Some("com.a.day".to_string()),
                    additional_resource_dirs: Some(vec![PathBuf::from("./day/src/main/res")]),
                    output_dir: None,
                    output_file: None,
                    version_code: None,
                    version_name: None,
                    package_id: None,
                },
            ]),
            package_id: None,
        };

        let app_b = AppConfig {
            base_dir: Some(PathBuf::from("./b/src/main")),
            resource_dir: None,
            manifest_path: None,
            package_name: "com.b".to_string(),
            additional_resource_dirs: None,
            output_dir: None,
            output_file: None,
            version_code: None,
            version_name: None,
            flavors: Some(vec![
                FlavorConfig {
                    name: "night".to_string(),
                    base_dir: None,
                    resource_dir: Some(PathBuf::from("./b/src/main/res-night")),
                    manifest_path: None,
                    package_name: Some("com.b.night".to_string()),
                    additional_resource_dirs: Some(vec![PathBuf::from("./night/src/main/res")]),
                    output_dir: None,
                    output_file: None,
                    version_code: None,
                    version_name: None,
                    package_id: None,
                },
                FlavorConfig {
                    name: "day".to_string(),
                    base_dir: None,
                    resource_dir: Some(PathBuf::from("./b/src/main/res-day")),
                    manifest_path: None,
                    package_name: Some("com.b.day".to_string()),
                    additional_resource_dirs: Some(vec![PathBuf::from("./day/src/main/res")]),
                    output_dir: None,
                    output_file: None,
                    version_code: None,
                    version_name: None,
                    package_id: None,
                },
            ]),
            package_id: None,
        };

        let multi_config = MultiAppConfig {
            base_dir: None,
            output_dir: PathBuf::from("./build"),
            output_file: None,
            android_jar: PathBuf::from("android.jar"),
            aapt2_path: None,
            aar_files: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            stable_ids_file: None,
            max_parallel_builds: None,
            package_id: None,
            apps: vec![app_a, app_b],
        };

        // Convert to BuildConfigs
        let configs = multi_config.into_build_configs();
        
        // Should have 4 configs: 2 apps × 2 flavors each
        assert_eq!(configs.len(), 4);
        
        // Extract common dependencies
        let common_deps = extract_common_dependencies(&configs);

        // Should find 2 common dependencies:
        // - ./night/src/main/res (used by app_a.night and app_b.night)
        // - ./day/src/main/res (used by app_a.day and app_b.day)
        assert_eq!(common_deps.len(), 2);
        
        // Check that both night and day resources are found
        let night_dep = common_deps.iter().find(|d| d.resource_dir == PathBuf::from("./night/src/main/res"));
        let day_dep = common_deps.iter().find(|d| d.resource_dir == PathBuf::from("./day/src/main/res"));
        
        assert!(night_dep.is_some(), "Should find ./night/src/main/res as common dependency");
        assert!(day_dep.is_some(), "Should find ./day/src/main/res as common dependency");
        
        // Each common dependency should be used by 2 configs
        assert_eq!(night_dep.unwrap().dependent_configs.len(), 2);
        assert_eq!(day_dep.unwrap().dependent_configs.len(), 2);
    }
}
