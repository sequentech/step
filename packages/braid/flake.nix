# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Flake to build rust library";

  # input
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };

  # output function of this flake
  outputs = { self, nixpkgs, flake-utils, rust-overlay, flake-compat }:
    flake-utils.lib.eachDefaultSystem (
      system:
        let
          overlays = [ (import rust-overlay) ];
          # pkgs is just the nix packages
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          
          rust-system = pkgs.rust-bin.stable."1.96.0".default.override {
            targets = [ "wasm32-unknown-unknown" ];
            extensions = [ "rust-src" ];
          };
          # Pin wasm-bindgen-cli to match the wasm-bindgen crate version in Cargo.toml (=0.2.104)
          # The CLI and crate versions must match exactly
          wasm-bindgen-cli-pinned = pkgs.rustPlatform.buildRustPackage rec {
            pname = "wasm-bindgen-cli";
            version = "0.2.104";
            src = builtins.fetchTarball {
              url = "https://crates.io/api/v1/crates/${pname}/${version}/download";
              sha256 = "00bv402z5n47f7l582xmanaxraacwg2pcm6rvlcify1bn9mvwign";
            };
            cargoHash = "sha256-V0AV5jkve37a5B/UvJ9B3kwOW72vWblST8Zxs8oDctE=";
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ]
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.curl ];
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