use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use anyhow::Result;

use crate::types::BuildConfig;

/// Represents a build configuration with its index for ordering
#[derive(Debug, Clone)]
pub struct ConfigWithIndex {
    pub index: usize,
    pub config: BuildConfig,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_config() {
        let configs = vec![BuildConfig::default_config()];
        let (independent, dependent) = group_configs_by_dependencies(configs).unwrap();
        assert_eq!(independent.len(), 1);
        assert_eq!(dependent.len(), 0);
    }

    #[test]
    fn test_independent_configs() {
        let config1 = BuildConfig {
            resource_dir: PathBuf::from("./res1"),
            manifest_path: PathBuf::from("./AndroidManifest.xml"),
            output_dir: PathBuf::from("./build1"),
            package_name: "com.example.app1".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };
        
        let config2 = BuildConfig {
            resource_dir: PathBuf::from("./res2"),
            manifest_path: PathBuf::from("./AndroidManifest.xml"),
            output_dir: PathBuf::from("./build2"),
            package_name: "com.example.app2".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };
        
        let configs = vec![config1, config2];
        let (independent, dependent) = group_configs_by_dependencies(configs).unwrap();
        
        // Both should be independent as they don't share resources
        assert_eq!(independent.len(), 2);
        assert_eq!(dependent.len(), 0);
    }

    #[test]
    fn test_dependent_configs() {
        // Base config
        let base_config = BuildConfig {
            resource_dir: PathBuf::from("./base/res"),
            manifest_path: PathBuf::from("./base/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.base".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };

        // Feature config that depends on base
        let feature_config = BuildConfig {
            resource_dir: PathBuf::from("./feature/res"),
            manifest_path: PathBuf::from("./feature/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.feature".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: Some(vec![PathBuf::from("./base/res")]),
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };

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
        let base_config = BuildConfig {
            resource_dir: PathBuf::from("./base/res"),
            manifest_path: PathBuf::from("./base/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.base".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };

        // Feature1 depends on base
        let feature1_config = BuildConfig {
            resource_dir: PathBuf::from("./feature1/res"),
            manifest_path: PathBuf::from("./feature1/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.feature1".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: Some(vec![PathBuf::from("./base/res")]),
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };

        // Feature2 also depends on base
        let feature2_config = BuildConfig {
            resource_dir: PathBuf::from("./feature2/res"),
            manifest_path: PathBuf::from("./feature2/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.feature2".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: Some(vec![PathBuf::from("./base/res")]),
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };

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
        let independent_config = BuildConfig {
            resource_dir: PathBuf::from("./independent/res"),
            manifest_path: PathBuf::from("./independent/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.independent".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };

        // Base config
        let base_config = BuildConfig {
            resource_dir: PathBuf::from("./base/res"),
            manifest_path: PathBuf::from("./base/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.base".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };

        // Feature depends on base
        let feature_config = BuildConfig {
            resource_dir: PathBuf::from("./feature/res"),
            manifest_path: PathBuf::from("./feature/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.feature".to_string(),
            android_jar: PathBuf::from("android.jar"),
            aar_files: None,
            aapt2_path: None,
            incremental: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: Some(vec![PathBuf::from("./base/res")]),
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        };

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
}
