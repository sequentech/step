# SPDX-FileCopyrightText: 2021 Eduardo Robles <edulix@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
{
  description = "Flake to test rust code";

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.nixpkgs.url = "nixpkgs/nixos-24.05";
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
          .nightly
          ."2022-07-31"
          .default
          .override {
              extensions = [ "rust-src" ];
               ${if (builtins.length targets) > 0 then "targets" else null} = targets;

          };
        rust-wasm = configureRustTargets [ "wasm32-unknown-unknown" ];
        rust-system  = configureRustTargets [];
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
            pkgs.wasm-bindgen-cli
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
        };

      }
    );
}
