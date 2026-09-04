# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Flake for test rust code";

  # input
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.nixpkgs.url = "nixpkgs/nixos-25.05";
  inputs.nixpkgs-crates.url = "nixpkgs/nixos-26.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };
  inputs.devenv.url = "github:cachix/devenv";

  # output function of this flake
  outputs = { self, nixpkgs, nixpkgs-crates, devenv, flake-utils, rust-overlay, flake-compat }:
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
          configureRustTargets = targets : pkgs
            .rust-bin
            .stable
            ."1.96.0"
            .default
            .override {
                extensions = [ "rust-src" ];
                ${if (builtins.length targets) > 0 then "targets" else null} = targets;

            };
          rust-wasm = configureRustTargets [ "wasm32-unknown-unknown" ];
          rust-system  = configureRustTargets [];

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
                  outputHashes = {
                  "strand-0.2.0" = "sha256-MBld1vxcFQ8IffKa4o0p6s++KQavYB7tcrQ4XpUxRoY=";
                };
              };
              postPatch = ''
                  cp ${./Cargo.lock.copy} Cargo.lock
              '';
          };
          buildRustPackageWithCargo = cargoArgs: pkgs.rustPlatform.buildRustPackage (cargoPatches // cargoArgs);

        # resulting packages of the flake
        in rec {
          packages.sequent-core-wasm = buildRustPackageWithCargo {
            pname = "sequent-core-wasm";
            version = "0.0.1";
            src = ./.;
            nativeBuildInputs = [
              rust-wasm
              pkgs.nodePackages.npm
              pkgs.binaryen
              pkgs.wasm-pack
              wasm-bindgen-cli-pinned
              pkgs.libiconv
              pkgs.m4

              # Add all the necessary LLVM/Clang packages
              pkgs.llvmPackages_19.clang-unwrapped
              pkgs.llvmPackages_19.llvm
              pkgs.llvmPackages_19.libclang
            ];
            buildPhase = ''
              echo 'Build: wasm-pack build'
              wasm-pack build --out-name index --release --target web --features=wasmtest,default_features
            '';
            installPhase = "
              # set HOME temporarily to fix npm pack
              mkdir -p $out/temp_home
              export HOME=$out/temp_home
              echo 'Install: wasm-pack pack'
              wasm-pack -v pack .
              rm -Rf $out/temp_home
              cp pkg/sequent-core-*.tgz $out
              ";
          };

          packages.sequent-core-lib = buildRustPackageWithCargo {
            pname = "sequent-core-lib";
            version = "0.0.1";
            src = ./.;
            nativeBuildInputs = [
              rust-system
            ];
          };
          # sequent-core is the default package
          defaultPackage = packages.sequent-core-wasm;

          # configure the dev shell
          devShell = (
            pkgs.mkShell.override { stdenv = pkgs.clangStdenv; }
          ) {
            nativeBuildInputs =
              defaultPackage.nativeBuildInputs;
            buildInputs =
              with pkgs; [
                # Your existing tools
                bash
                reuse
                cargo-deny
                ack
                wasm-pack

                # Add these two lines for browser testing
                firefox
                geckodriver
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
              # Debug: Print the paths to verify they exist
              echo "Clang resource dir: $CLANG_RESOURCE_DIR"
              echo "Libclang include dir: $LIBCLANG_INCLUDE"
              if [ -f "$LIBCLANG_INCLUDE/stddef.h" ]; then
                echo "Found stddef.h at: $LIBCLANG_INCLUDE/stddef.h"
              else
                echo "stddef.h not found in $LIBCLANG_INCLUDE"
              fi
            '';
          };
        }
    );
}