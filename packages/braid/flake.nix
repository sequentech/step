# SPDX-FileCopyrightText: 2021 Eduardo Robles <edu@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Flake to build rust library";

  # input
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
          
          rust-system = pkgs.rust-bin.stable.latest.default;
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
            buildInputs = 
              packages.braid.buildInputs ++
              [ pkgs.bash pkgs.reuse pkgs.cargo-deny ]; 
            nativeBuildInputs = packages.braid.nativeBuildInputs;
          };
        }
    );
}