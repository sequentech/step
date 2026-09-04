# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Flake to build rust library";

  # input
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  inputs.nixpkgs-crates.url = "nixpkgs/nixos-26.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };

  # output function of this flake
  outputs = { self, nixpkgs, nixpkgs-crates, flake-utils, rust-overlay, flake-compat }:
    flake-utils.lib.eachDefaultSystem (
      system:
        let
          overlays = [ (import rust-overlay) ];
          # pkgs is just the nix packages
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          pkgsCrates = import nixpkgs-crates {
            inherit system;
          };

          rust-system = pkgs.rust-bin.stable."1.96.0".default.override {
            targets = [ "wasm32-unknown-unknown" ];
            extensions = [ "rust-src" ];
          };

          # wasm-bindgen has no semver guarantee, so the CLI must match the crate
          # version exactly (=0.2.123). Not in nixpkgs, so this is a source build.
          # Built entirely against pkgsCrates (nixos-26.05): the crate vendorer in
          # our main pin sends a default python-requests User-Agent, which crates.io
          # answers with HTTP 403. The rustc that builds the CLI is 26.05's and need
          # not match our 1.96.0 — only the CLI *version* must match the crate.
          wasm-bindgen-cli-pinned = pkgsCrates.rustPlatform.buildRustPackage rec {
            pname = "wasm-bindgen-cli";
            # Pinned to the wasm-bindgen crate version both Cargo workspaces use
            # (packages/Cargo.toml and packages/wbraid/Cargo.toml: =0.2.123).
            version = "0.2.123";
            cargoHash = "sha256-d7x6gtx5OqEE4MyT6yjYn/qtgjx7GroTpXJewnBV2dU=";
            src = builtins.fetchTarball {
              url = "https://static.crates.io/crates/${pname}/${pname}-${version}.crate";
              sha256 = "12xdns7cvnz0j26i9kryxggylsslkqs5l2b6lppfkv1bic8q0rya";
            };
            nativeBuildInputs = [ pkgsCrates.pkg-config ];
            buildInputs = [ pkgsCrates.openssl ]
              ++ pkgsCrates.lib.optionals pkgsCrates.stdenv.hostPlatform.isDarwin [ pkgsCrates.curl ];
            doCheck = false;
          };

          # see https://github.com/NixOS/nixpkgs/blob/master/doc/languages-frameworks/rust.section.md#importing-a-cargolock-file-importing-a-cargolock-file
          cargoPatches = {
              cargoLock = let
                  fixupLockFile = path: (builtins.readFile path);
              in {
                lockFileContents = fixupLockFile ./Cargo.lock.copy;
                  outputHashes = {};
              };
              postPatch = ''
                  cp ${./Cargo.lock.copy} Cargo.lock
              '';
          };
          buildRustPackageWithCargo = cargoArgs: pkgs.rustPlatform.buildRustPackage (cargoPatches // cargoArgs);

        # resulting packages of the flake
        in rec {
          packages.braid = buildRustPackageWithCargo {
            pname = "braid";
            version = "0.0.1";
            src = ./.;
            buildInputs = [
              pkgs.openssl
              rust-system
            ] ++ pkgs.lib.lists.optionals pkgs.stdenv.isDarwin [ pkgs.darwin.apple_sdk.frameworks.Security ];
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.m4
            ];
          };
          # braid is the default package
          defaultPackage = packages.braid;

          # configure the dev shell
          devShell = (
            pkgs.mkShell.override { stdenv = pkgs.clangStdenv; }
          ) {
            # Put rust-system first to ensure nightly is used
            buildInputs = [
              rust-system
              pkgs.openssl
              pkgs.bash
              pkgs.reuse
              pkgs.cargo-deny
            ] ++ pkgs.lib.lists.optionals pkgs.stdenv.isDarwin [ pkgs.darwin.apple_sdk.frameworks.Security ];
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.m4
              pkgs.nodePackages.npm
              wasm-bindgen-cli-pinned

              # Add all the necessary LLVM/Clang packages
              pkgs.llvmPackages_19.clang-unwrapped
              pkgs.llvmPackages_19.llvm
              pkgs.llvmPackages_19.libclang
            ];
            shellHook = ''
              export CC=${pkgs.llvmPackages_19.clang-unwrapped}/bin/clang
              export CXX=${pkgs.llvmPackages_19.clang-unwrapped}/bin/clang++
              export AR=${pkgs.llvmPackages_19.llvm}/bin/llvm-ar
              export CC_wasm32_unknown_unknown=${pkgs.llvmPackages_19.clang-unwrapped}/bin/clang
              # Nix hardening flags are not supported when compiling C code for WebAssembly
              export NIX_HARDENING_ENABLE=""
              # Set up the clang resource directory properly
              CLANG_MAJOR_VERSION="19"
              CLANG_RESOURCE_DIR="${pkgs.llvmPackages_19.clang-unwrapped}/lib/clang/$CLANG_MAJOR_VERSION"
              export CLANG_RESOURCE_DIR
              # Use libclang's include directory which has the standard headers
              LIBCLANG_INCLUDE="${pkgs.llvmPackages_19.libclang.lib}/lib/clang/$CLANG_MAJOR_VERSION/include"
              export LIBCLANG_INCLUDE
              export CFLAGS_wasm32_unknown_unknown="-isystem $LIBCLANG_INCLUDE -resource-dir $CLANG_RESOURCE_DIR -O3 -ffunction-sections -fdata-sections -fno-exceptions"
              export CPPFLAGS="-isystem $LIBCLANG_INCLUDE -resource-dir $CLANG_RESOURCE_DIR"
            '';
          };
        }
    );
}