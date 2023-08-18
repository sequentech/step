# SPDX-FileCopyrightText: 2022 Eduardo Robles <edulix@sequentech.io>
# SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Sequent bulletin-board Nix flake";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.nixpkgs.url = "nixpkgs/nixos-22.11";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };
  inputs.crane = {
    url = "github:ipetkov/crane";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  
  outputs = { self, nixpkgs, flake-utils, rust-overlay, flake-compat, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let 
        packageName = "bulletin-board";
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { 
          inherit system overlays;
        };
        configureRustTargets = targets : pkgs
          .rust-bin
          .nightly
          ."2023-02-15"
          .default
          .override {
            extensions = [ "rust-src" ];
              ${
                if (builtins.length targets) > 0 
                then "targets" 
                else null
              } = targets;
          };
        rust-system = configureRustTargets [];

        craneLib = (crane.mkLib pkgs).overrideToolchain rust-system;

        goFilter = path: _type: builtins.match ".*\.(go|mod|sum)$" path != null;
        grpcFilter = path: _type: builtins.match ".*\.proto$" path != null;
        sourcesFilter = path: type:
          (goFilter path type) || 
          (grpcFilter path type) || 
          (craneLib.filterCargoSources path type);

        commonArgs = {
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = sourcesFilter;
          };
          # allow CI task to fetch git dependencies

          # Build time dependencies
          nativeBuildInputs = with pkgs; [
            pkgs.protobuf
            pkgs.trillian
            pkgs.go
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "${packageName}-deps";
        });

        bulletinBoard = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = packageName;
          version = "0.0.1";
          cargoVendorDir = craneLib.vendorCargoDeps {
            src = commonArgs.src;
          };
        });
      in rec {
        # Executed by `nix build .#<name>`
        packages.bulletin-board = bulletinBoard;

        # Executed by `nix build .`
        defaultPackage = self.packages.${system}.bulletin-board;

        # Used by `nix develop`
        devShell = (
          pkgs.mkShell.override { stdenv = pkgs.clangStdenv; }
        ) {
          # Build time dependencies
          nativeBuildInputs = 
            defaultPackage.nativeBuildInputs; 

          # Run time dependencies
          buildInputs = 
            [
              pkgs.bash
              pkgs.reuse

              # Useful CURL-like tool for development
              pkgs.grpcurl

              # Go build dependency
              pkgs.go

              # Go tools used by VS Code Go extensions
              pkgs.go-outline
              pkgs.gopls

              # To be able to use vim in the terminal
              pkgs.vim

              # Used for the examples configuration
              pkgs.yq
              pkgs.tree

              # Convenience during development
              pkgs.ack

              # For docusaurus
              pkgs.yarn
              pkgs.protoc-gen-doc
            ];
        };

        # TODO: Executed by `nix flake check`
        # checks =
        #
        # TODO: Executed by `nix run .`
        # apps.default = 
      }
    );

  nixConfig = {
    extra-substituters = [ "https://sequentech.cachix.org" ];
    extra-trusted-public-keys = [ "sequentech.cachix.org-1:mmoak2RFNZkQjHHpKn/NbsBrznWqvq8COKqaVOI6ahM=" ];
  };
}
