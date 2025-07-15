# SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Flake for test rust code";

  # input
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };
  inputs.devenv.url = "github:cachix/devenv";

  # output function of this flake
  outputs = { self, nixpkgs, devenv, flake-utils, rust-overlay, flake-compat }:
    flake-utils.lib.eachDefaultSystem (
      system:
        let
          overlays = [ (import rust-overlay) ];
          # pkgs is just the nix packages
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          configureRustTargets = targets : pkgs
            .rust-bin
            .nightly
            .latest
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
              pkgs.wasm-bindgen-cli
              pkgs.libiconv
              pkgs.m4
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
              with pkgs; [ bash reuse cargo-deny ack wasm-pack ];
          };
        }
    );
}