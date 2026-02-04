# Testing Guide for Array Mode Configuration

This document describes the test suite for the array mode configuration feature in ASB.

## Test Structure

The test suite is organized into three categories:

### 1. Unit Tests for Dependency Resolution (`src/dependency.rs`)

These tests verify the dependency resolution logic:

- **`test_single_config`**: Verifies handling of a single configuration
- **`test_independent_configs`**: Tests two independent configs that should build in parallel
- **`test_dependent_configs`**: Tests configs with dependencies (base + feature)
- **`test_multiple_features_depending_on_base`**: Tests multiple features depending on a common base
- **`test_mixed_independent_and_dependent_configs`**: Tests a mix of independent and dependent configs

### 2. Unit Tests for Configuration Loading (`src/types.rs`)

These tests verify JSON parsing and deserialization:

- **`test_load_single_config`**: Tests loading a single configuration object
- **`test_load_array_config`**: Tests loading an array of configurations
- **`test_load_array_config_with_dependencies`**: Tests array config with `additionalResourceDirs`

### 3. Integration Tests (`tests/integration_test.rs`)

These tests verify end-to-end functionality:

- **`test_load_single_config_from_json`**: Full single config with all common fields
- **`test_load_array_config_from_json`**: Full array config with version info
- **`test_array_config_with_additional_resources`**: Tests dependency declaration via `additionalResourceDirs`
- **`test_backward_compatibility_single_to_array`**: Verifies backward compatibility fallback
- **`test_config_with_all_optional_fields`**: Tests all optional configuration fields

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Only Unit Tests

```bash
cargo test --lib
```

### Run Only Integration Tests

```bash
cargo test --test integration_test
```

### Run Specific Test

```bash
cargo test test_dependent_configs
```

### Run Tests with Output

```bash
cargo test -- --nocapture
```

## Test Coverage

The test suite covers:

✅ **Configuration Loading**

- Single object format (backward compatibility)
- Array format (new feature)
- Fallback from array to single object parsing

✅ **Dependency Resolution**

- Independent configurations (parallel build)
- Dependent configurations (sequential build)
- Mixed independent and dependent configurations
- Multiple features depending on common base
- Topological sort correctness

✅ **Configuration Fields**

- Required fields: `resourceDir`, `manifestPath`, `outputDir`, `packageName`, `androidJar`
- Optional fields: `aapt2Path`, `aarFiles`, `incremental`, `cacheDir`, `versionCode`, `versionName`
- Array-specific fields: `additionalResourceDirs`, `compiledDir`, `stableIdsFile`
- Multi-app specific fields: `maxParallelBuilds` (controls how many apps can be built in parallel)

✅ **Edge Cases**

- Empty configurations
- Single configuration in array mode
- Circular dependencies (should be detected and fail)
- Missing dependencies

## Test Results

Current test status:

```
running 8 tests (unit tests - dependency.rs)
test dependency::tests::test_single_config ... ok
test dependency::tests::test_independent_configs ... ok
test dependency::tests::test_dependent_configs ... ok
test dependency::tests::test_multiple_features_depending_on_base ... ok
test dependency::tests::test_mixed_independent_and_dependent_configs ... ok

running 3 tests (unit tests - types.rs)
test types::tests::test_load_single_config ... ok
test types::tests::test_load_array_config ... ok
test types::tests::test_load_array_config_with_dependencies ... ok

running 5 tests (integration tests)
test test_load_single_config_from_json ... ok
test test_load_array_config_from_json ... ok
test test_array_config_with_additional_resources ... ok
test test_backward_compatibility_single_to_array ... ok
test test_config_with_all_optional_fields ... ok

test result: ok. 21 passed; 0 failed
```

## Adding New Tests

To add new tests:

1. **For dependency resolution logic**: Add tests to `src/dependency.rs` in the `#[cfg(test)] mod tests` section
2. **For configuration parsing**: Add tests to `src/types.rs` in the `#[cfg(test)] mod tests` section
3. **For integration/end-to-end tests**: Add tests to `tests/integration_test.rs`

### Example Test Template

```rust
#[test]
fn test_my_feature() {
    // Setup
    let config = BuildConfig { /* ... */ };

    // Execute
    let result = some_function(config);

    // Assert
    assert_eq!(result.something, expected_value);
}
```

## Continuous Integration

These tests should be run as part of CI/CD pipeline:

```yaml
# Example GitHub Actions workflow
- name: Run tests
  run: cargo test --all
```

## Manual Testing

For manual end-to-end testing, use the example configurations:

```bash
# Test single config (backward compatibility)
cd examples/simple-skin
cargo run --release -- build

# Test array config without dependencies
cd examples/array-config
cargo run --release -- build

# Test array config with dependencies
cd examples/array-config-deps
cargo run --release -- build
```

## Troubleshooting

If tests fail:

1. Check that all dependencies are installed: `cargo build`
2. Ensure you're using the correct Rust version: `rustc --version` (1.70+)
3. Clean build artifacts: `cargo clean && cargo test`
4. Check for environment-specific issues (paths, permissions, etc.)

## Performance Testing

While not included in the automated test suite, manual performance testing can be done:

```bash
# Time independent config builds (should be ~0.12s)
time cargo run --release -- build -c examples/array-config/asb.config.json

# Time dependent config builds (should be ~0.27s)
time cargo run --release -- build -c examples/array-config-deps/asb.config.json
```
