---
sidebar_label: Updating Rust Version
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Updating Rust Version

This guide explains how to update the Rust version across the entire Step Repository. It's critical that all locations are updated together to ensure consistency across development environments, CI/CD pipelines, and production builds.

## Overview

The Step Repository uses **Rust stable version 1.96.0** throughout the entire codebase. We maintain a single, consistent version across:

- GitHub Actions (CI/CD pipelines)
- All Dockerfiles (development and production)
- Nix development environment (devenv.nix)

## Why Synchronization Matters

Rust versions must be synchronized across all systems to ensure:

1. **Reproducible builds** - Same code compiles identically everywhere
2. **Consistent behavior** - No surprises from compiler version differences
3. **Dependency compatibility** - All dependencies work with the chosen version
4. **CI/CD reliability** - Tests and builds don't fail due to version mismatches
5. **Developer experience** - Local development matches production

## Complete Update Checklist

When updating the Rust version, you MUST update it in all of the following locations:

### 1. GitHub Actions Workflows

Update the `toolchain` version in these workflow files:

- **`.github/workflows/tests.yml`** (line 43)
  ```yaml
  - name: Install Rust
    uses: dtolnay/rust-toolchain@stable
    with:
      profile: minimal
      toolchain: 1.90.0  # ← Update this
      components: rustfmt
      target: x86_64-unknown-linux-musl
  ```

- **`.github/workflows/lint_prettify.yml`** (line 102)
  ```yaml
  - name: Install Rust
    uses: dtolnay/rust-toolchain@stable
    with:
      profile: minimal
      toolchain: 1.90.0  # ← Update this
      components: rustfmt
      target: x86_64-unknown-linux-musl
  ```

- **`.github/workflows/step_cli_build.yml`** (line 38)
  ```yaml
  - name: Install Rust
    uses: dtolnay/rust-toolchain@stable
    with:
      profile: minimal
      toolchain: 1.90.0  # ← Update this
      components: rustfmt
      target: x86_64-unknown-linux-musl
  ```

### 2. Dockerfiles

Update the `FROM rust:X.Y.Z` line in **all** Dockerfiles:

- **`packages/Dockerfile.cargo-packages`** (line 10)
  ```dockerfile
  FROM rust:1.90.0-slim-bookworm  # ← Update this
  ```

- **`packages/braid/Dockerfile`** (line 5)
  ```dockerfile
  FROM rust:1.90.0-slim-bookworm  # ← Update this
  ```

- **`packages/braid/Dockerfile.prod`**
- **`packages/braid/Dockerfile.prod-vstl-dependencies`**
- **`packages/windmill/Dockerfile`**
- **`packages/windmill/Dockerfile.prod`**
- **`packages/windmill/Dockerfile.prod-vstl-dependencies`**
- **`packages/harvest/Dockerfile`**
- **`packages/harvest/Dockerfile.prod`**
- **`packages/harvest/Dockerfile.prod-vstl-dependencies`**
- **`packages/b3/Dockerfile.prod`**
- **`packages/b3/Dockerfile.prod-vstl-dependencies`**
- **`packages/e2e/src/mock_server/Dockerfile.prod`**
- **`packages/loadtesting/Dockerfile`**
- **`packages/Dockerfile.immudb-init-vstl-dependencies`**
- **`packages/Dockerfile.immudb-init.prod`**
- **`packages/step-cli/Dockerfile`**
- **`packages/orare/doc_renderer/Dockerfile`**

**Important**: Remove any `rustup toolchain install nightly-*` commands if present. We only use stable Rust.

### 3. Nix Configuration

Update the pinned Rust version in the Nix development environment:

- **`devenv.nix`** (lines 4–15) — update the `rust-overlay` commit and the version string in `rust-bin.stable`:
  ```nix
  let
    rustOverlay = import (builtins.fetchTarball {
      # Replace with a commit from oxalica/rust-overlay made after the target Rust release
      url = "https://github.com/oxalica/rust-overlay/archive/<COMMIT_HASH>.tar.gz";
      # Leave as "" or a dummy hash first, then paste the hash Nix reports
      sha256 = "<SHA256_HASH>";
    });

    pkgs' = pkgs.extend rustOverlay;

    # Pinned to stable version X.Y.Z
    rustStable = pkgs'.rust-bin.stable."X.Y.Z".default.override {
      targets    = [ "wasm32-unknown-unknown" "wasm32-wasip1" "wasm32-wasip2" ];
      extensions = [ "rust-src" "rust-analyzer-preview" ];
    };
  in
  ```

**How to update the `sha256` hash**: set it to `""` (empty string), run `devenv shell`, and Nix will fail with the actual hash — paste that value back in.

**Note**: Because `rust-overlay` is pinned to a specific commit, the Rust version is fully reproducible and does **not** depend on the nixpkgs snapshot. The version in `rust-bin.stable."X.Y.Z"` is the authoritative source — this is the same version number that must be used in GitHub Actions and Dockerfiles.

- **`packages/sequent-core/flake.nix`** — used to build the `sequent-core` WASM package (`nix develop` + `wasm-pack build`, see `.github/workflows/build_wasm.yml`). Update `configureRustTargets` to point at `rust-bin.stable."X.Y.Z"`:
  ```nix
  configureRustTargets = targets : pkgs
    .rust-bin
    .stable
    ."X.Y.Z"
    .default
    .override {
        extensions = [ "rust-src" ];
        ${if (builtins.length targets) > 0 then "targets" else null} = targets;
    };
  ```
  This flake's `rust-overlay` input is pinned via `packages/sequent-core/flake.lock`. If the target Rust version isn't available yet in that pinned revision, refresh it with:
  ```bash
  cd packages/sequent-core && nix flake update rust-overlay
  ```

## Step-by-Step Update Process

### 1. Update Nix First

**Always start by updating the Nix environment first**, as this pins the Rust version for local development.

#### Update devenv.nix

In `devenv.nix`, two things must be updated together:

1. **`rust-overlay` commit** — find a recent commit from [oxalica/rust-overlay](https://github.com/oxalica/rust-overlay) that was pushed *after* the target Rust release, and update the `url` and `sha256` fields in the `rustOverlay` block.
2. **Version string** — update the version number inside `pkgs'.rust-bin.stable."X.Y.Z"`.

```nix
rustOverlay = import (builtins.fetchTarball {
  url = "https://github.com/oxalica/rust-overlay/archive/<NEW_COMMIT_HASH>.tar.gz";
  sha256 = "";  # leave empty first — Nix will print the correct hash
});

# ...

rustStable = pkgs'.rust-bin.stable."X.Y.Z".default.override {
  targets    = [ "wasm32-unknown-unknown" "wasm32-wasip1" "wasm32-wasip2" ];
  extensions = [ "rust-src" "rust-analyzer-preview" ];
};
```

#### Get the sha256 Hash

After setting the new commit URL, leave `sha256 = ""` and run:

```bash
devenv shell
```

Nix will fail and print the actual hash. Paste it into `sha256`, then run `devenv shell` again — it should succeed.

#### Verify the Rust Version in Nix

```bash
# Enter the devenv shell
devenv shell

# Check the actual Rust version
rustc --version
# Output example: rustc 1.90.0 (abc123def 2024-10-01)
```

The version shown must match the string in `rust-bin.stable."X.Y.Z"`. **This is the version you should use for all other files** (GitHub Actions and Dockerfiles).

### 2. Check Current Versions in Other Files

Now identify the Rust versions currently used in other parts of the codebase:

```bash
# Search for Rust versions in Dockerfiles
grep -r "FROM rust:" packages/

# Search for Rust versions in GitHub Actions
grep -r "toolchain:" .github/workflows/
```

Compare these versions with what you found in Nix. They should all match.

### 3. Decide on Target Version

If you need to update to a newer Rust version:

1. **Choose the target version** - Pick the stable Rust version you want (e.g. `1.91.0`)
2. **Find a `rust-overlay` commit** - Browse [oxalica/rust-overlay](https://github.com/oxalica/rust-overlay) commits and pick one made after the target release date
3. **Review breaking changes** - Check https://www.rust-lang.org/ and release notes
4. **Ensure compatibility** - Verify existing dependencies work with the target version

With `rust-overlay`, the version is fully explicit — `devenv.nix` is the authoritative source and Dockerfiles / GitHub Actions must match it.

### 4. Update All Other Files to Match Nix

Update all files to use the same version that Nix is providing:

1. **Update GitHub Actions** - Set `toolchain:` to the version from Nix
2. **Update Dockerfiles** - Set `FROM rust:X.Y.Z` to the version from Nix

This ensures consistency: your local development environment (Nix), CI/CD (GitHub Actions), and production builds (Dockerfiles) all use the same Rust version.

### 5. Use Search and Replace

To update all versions at once:

```bash
# Update stable version in GitHub Actions
find .github/workflows/ -name "*.yml" -type f -exec sed -i 's/toolchain: 1.90.0/toolchain: X.Y.Z/g' {} +

# Update stable version in Dockerfiles
find packages/ -name "Dockerfile*" -type f -exec sed -i 's/FROM rust:1.90.0/FROM rust:X.Y.Z/g' {} +

# Remove any nightly references (if any exist)
find packages/ -name "Dockerfile*" -type f -exec sed -i '/rustup toolchain install nightly/d' {} +
find packages/ -name "Dockerfile*" -type f -exec sed -i '/rustup default nightly/d' {} +
```

**Warning**: Always review changes after automated replacements!

### 6. Test the Changes

After updating, test thoroughly:

#### Local Testing

```bash
# Rebuild devenv to use new Rust version
devenv shell

# Verify Rust version
rustc --version
# Should output: rustc X.Y.Z (expected version)

cargo --version

# Run tests locally
cd packages/
cargo test
cargo fmt -- --check
```

#### CI Testing

1. Commit your changes to a feature branch
2. Open a pull request
3. Wait for all CI checks to pass:
   - Tests workflow
   - Lint & Prettify workflow
   - Step CLI build workflow

#### Docker Testing

Build a few key Docker images to verify:

```bash
# Test building services
cd packages/
docker build -f braid/Dockerfile -t test-braid .
docker build -f windmill/Dockerfile -t test-windmill .
docker build -f Dockerfile.cargo-packages -t test-cargo .
```

### 7. Verify Consistency

After all updates, verify everything is in sync:

```bash
# Should show consistent versions across all files
grep -r "toolchain: 1\." .github/workflows/
grep -r "FROM rust:1\." packages/

# Should return NO results (no nightly versions should exist)
grep -r "nightly" packages/ | grep -i rust
```

## Reference: Complete File List

Here's the complete list of files that need updating:

### GitHub Actions (3 files)
- `.github/workflows/tests.yml`
- `.github/workflows/lint_prettify.yml`
- `.github/workflows/step_cli_build.yml`

### Nix Configuration (2 files)
- `devenv.nix`
- `packages/sequent-core/flake.nix` (plus `packages/sequent-core/flake.lock` if `rust-overlay` needs updating)

### Dockerfiles (19 files)
- `packages/Dockerfile.cargo-packages`
- `packages/braid/Dockerfile`
- `packages/braid/Dockerfile.prod`
- `packages/braid/Dockerfile.prod-vstl-dependencies`
- `packages/windmill/Dockerfile`
- `packages/windmill/Dockerfile.prod`
- `packages/windmill/Dockerfile.prod-vstl-dependencies`
- `packages/harvest/Dockerfile`
- `packages/harvest/Dockerfile.prod`
- `packages/harvest/Dockerfile.prod-vstl-dependencies`
- `packages/b3/Dockerfile.prod`
- `packages/b3/Dockerfile.prod-vstl-dependencies`
- `packages/e2e/src/mock_server/Dockerfile.prod`
- `packages/loadtesting/Dockerfile`
- `packages/Dockerfile.immudb-init-vstl-dependencies`
- `packages/Dockerfile.immudb-init.prod`
- `packages/step-cli/Dockerfile`
- `packages/orare/doc_renderer/Dockerfile`

## Best Practices

1. **Update regularly** - Stay reasonably current with stable Rust releases
2. **Test thoroughly** - Don't skip the testing steps
3. **Update together** - Never update just one location; always update all
4. **Document changes** - Note any code changes required for the new version
5. **Monitor CI** - Watch for failures in the automated pipeline
6. **Use stable only** - We don't use nightly features; stick to stable Rust
