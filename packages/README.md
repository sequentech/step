## 🧹 Linting & Code Quality Guide

This project enforces strict linting rules using Rust built-in lints and Clippy to ensure high code quality, safety, and maintainability.

### 🚀 Running Clippy

Open the terminal in the desire package and run lint checks locally with:

```bash
cd packages/sequent-core
cargo clippy --no-deps --all-features -- -A warnings
```

Explanation
--no-deps: Lints only this workspace (skips dependencies)
--all-features: Ensures all feature-gated code is checked
-- -A warnings: Allows warnings (only errors will fail)

- Note: This is convenient for development, but CI should be stricter.

### Lint Configuration

Lint rules are defined in Cargo.toml under [lints.*].

#### Rustdoc Lints

The `[lints.rustdoc]` section enforces documentation quality and correctness.

- `missing_crate_level_docs = "deny"`  
  Requires top-level crate documentation (e.g. `//!` or `#![doc = include_str!(...)]`).

- `broken_intra_doc_links = "deny"`  
  Prevents broken links between documentation items (e.g. invalid `[`TypeName`]` references).
  
#### Rust Lints

The `[lints.rust]` section configures core compiler lints to enforce safety, correctness, and good API design.

- `missing_docs = "deny"`  
  Requires documentation for all public items (structs, enums, functions, etc.).

- `unsafe_code = "forbid"`  
  Completely disallows usage of `unsafe` code anywhere in the crate.

- `private_interfaces = "warn"`  
  Warns when a public item exposes private types in its interface.

- `private_bounds = "warn"`  
  Warns when trait bounds reference private types that are not accessible to users.

- `unnameable_types = "warn"`  
  Warns when types are exposed that cannot be named or used properly outside the crate.

- `unexpected_cfgs = { level = "warn", check-cfg = ['cfg(coverage,coverage_nightly)'] }`  
  Warns about unknown or unexpected `cfg` conditions and ensures only allowed configuration flags (like `coverage`) are used.
  
#### Clippy Lints

The `[lints.clippy]` section configures additional lint rules provided by Clippy. In this project, these rules are set very strictly to enforce safer, clearer, and more maintainable Rust code.

- `missing_docs_in_private_items = "deny"`  
  Requires documentation not only for public items, but also for private structs, enums, functions, constants, and fields.

- `missing_errors_doc = "deny"`  
  Requires a `# Errors` section in the documentation of functions that return `Result`.

- `missing_panics_doc = "deny"`  
  Requires a `# Panics` section in the documentation of functions that may panic.

- `doc_markdown = "deny"`  
  Enforces proper Markdown formatting in documentation comments.

- `unwrap_used = "deny"`  
  Disallows use of `.unwrap()` because it may panic.

- `panic = "deny"`  
  Disallows explicit `panic!()` calls.

- `shadow_unrelated = "deny"`  
  Disallows reusing variable names in unrelated scopes when it may reduce readability.

- `print_stdout = "deny"`  
  Disallows `println!` and similar macros for standard output.

- `print_stderr = "deny"`  
  Disallows `eprintln!` and similar macros for standard error output.

- `indexing_slicing = "deny"`  
  Disallows direct indexing and slicing like `arr[i]` because they may panic at runtime.

- `missing_const_for_fn = "deny"`  
  Requires functions to be marked `const` when they can be.

- `future_not_send = "deny"`  
  Warns when async code creates futures that are not `Send`, which can break multi-threaded async execution.

- `arithmetic_side_effects = "deny"`  
  Flags arithmetic operations that may overflow or have unintended side effects.

- `suspicious = "deny"`  
  Enables Clippy’s suspicious code lints to catch code that is likely incorrect.

- `complexity = "deny"`  
  Enables lints that detect overly complex code patterns and suggests simpler alternatives.

- `style = "deny"`  
  Enforces idiomatic Rust style.

- `perf = "deny"`  
  Enables performance-related lints to catch inefficient code patterns.

- `pedantic = "deny"`  
  Enables a very strict set of extra Clippy lints for code quality, readability, and correctness. These are more opinionated than the default lints.