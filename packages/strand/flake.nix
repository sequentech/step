# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
{
  description = "Flake to test rust code";

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.nixpkgs.url = "nixpkgs/nixos-25.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };
  
  outputs = { self, nixpkgs, flake-utils, rust-overlay, flake-compat }:
    flake-utils.lib.eachDefaultSystem (system:
      let 
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { 
          inherit system overlays;
        };
        stdenv = pkgs.clangStdenv;
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
        # Pin wasm-bindgen-cli to match the wasm-bindgen crate version in Cargo.toml (=0.2.123)
        # The CLI and crate versions must match exactly. Built with the flake's own
        # toolchain: nixos-25.05's rustc (1.86) predates the 1.88 the CLI's dependencies need.
        rustPlatform-1_96 = pkgs.makeRustPlatform { cargo = rust-system; rustc = rust-system; };
        wasm-bindgen-cli-pinned = rustPlatform-1_96.buildRustPackage rec {
          pname = "wasm-bindgen-cli";
          version = "0.2.123";
          src = builtins.fetchTarball {
            # static CDN: crates.io's /api/v1 download endpoint answers nix's downloader with HTTP 403
            url = "https://static.crates.io/crates/${pname}/${pname}-${version}.crate";
            sha256 = "12xdns7cvnz0j26i9kryxggylsslkqs5l2b6lppfkv1bic8q0rya";
          };
          cargoHash = "sha256-d7x6gtx5OqEE4MyT6yjYn/qtgjx7GroTpXJewnBV2dU=";
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
            };
            postPatch = ''
                cp ${./Cargo.lock.copy} Cargo.lock
            '';
        };
        buildRustPackageWithCargo = cargoArgs: pkgs.rustPlatform.buildRustPackage (cargoPatches // cargoArgs);
      in rec {
        packages.strand-wasm = buildRustPackageWithCargo {
          pname = "strand-wasm";
          version = "0.0.1";
          src = ./.;
          nativeBuildInputs = [
            rust-wasm
            pkgs.nodePackages.npm
            pkgs.binaryen
            pkgs.wasm-pack
            wasm-bindgen-cli-pinned

            # Add all the necessary LLVM/Clang packages
            pkgs.llvmPackages_19.clang-unwrapped
            pkgs.llvmPackages_19.llvm
            pkgs.llvmPackages_19.libclang
          ];
          buildPhase = ''
            echo 'Build: wasm-pack build'
            wasm-pack build --mode no-install --out-name index --release --target web --features=wasmtest
          '';
          installPhase = "
            # set HOME temporarily to fix npm pack
            mkdir -p $out/temp_home
            export HOME=$out/temp_home
            echo 'Install: wasm-pack pack'
            wasm-pack -v pack .
            rm -Rf $out/temp_home
            cp pkg/strand-*.tgz $out
            ";
        };
        packages.strand-lib = buildRustPackageWithCargo {
          pname = "strand-lib";
          version = "0.0.1";
          src = ./.;
          nativeBuildInputs = [
            rust-system
          ];
        };
        defaultPackage = self.packages.${system}.strand-wasm;

        # configure the dev shell
        devShell = (
          pkgs.mkShell.override { stdenv = pkgs.clangStdenv; }
        ) {
          nativeBuildInputs = 
            defaultPackage.nativeBuildInputs; 
          buildInputs =
            [
              pkgs.bash
              pkgs.reuse
              pkgs.cargo-deny
              pkgs.clippy
              pkgs.pkg-config
              pkgs.openssl
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
