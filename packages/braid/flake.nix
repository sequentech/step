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
          
          rust-system = pkgs.rust-bin.nightly."2025-01-29".default.override {
            targets = [ "wasm32-unknown-unknown" ];
            extensions = [ "rust-src" ];
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

          # Pin wasm-bindgen-cli to match the wasm-bindgen crate version in Cargo.toml (=0.2.104)
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
              wasm-bindgen-cli-pinned
              pkgs.nodejs
            ] ++ pkgs.lib.lists.optionals pkgs.stdenv.isDarwin [ pkgs.darwin.apple_sdk.frameworks.Security ];
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.m4
            ];
          };
        }
    );
}