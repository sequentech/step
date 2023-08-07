# SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Flake for ui-essentials";

  # input
  inputs.nixpkgs.url = "nixpkgs/nixos-22.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };

  # output function of this flake
  outputs = { self, nixpkgs, flake-utils, flake-compat }:
    flake-utils.lib.eachDefaultSystem (
      system:
        let 
          overlays = [];
          # pkgs is just the nix packages
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          pname = "ui-essentials";
          version = "0.0.1";

        # resulting packages of the flake
        in rec {
          packages.ui-essentials = pkgs.mkYarnPackage {
            inherit pname;
            inherit version;
            packageJSON = ./package.json;
            yarnLock = ./yarn.lock;
            yarnNix = ./yarn.nix;
            src = ./.;
            buildPhase = ''
              export HOME=$(mktemp -d)
              yarn --offline build
              ls -lah
            '';
          };

          # ui-essentials is the default package
          defaultPackage = packages.ui-essentials;

          # configure the dev shell
          devShell = (
            pkgs.mkShell.override { stdenv = pkgs.clangStdenv; }
          ) {
            buildInputs = 
              [ pkgs.bash pkgs.reuse pkgs.cargo-deny pkgs.ack pkgs.htop pkgs.yarn2nix pkgs.yarn ]; 
          };
        }
    );
}