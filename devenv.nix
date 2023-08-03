{ pkgs, ... }:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = [
    pkgs.git
    pkgs.hasura-cli
    pkgs.reuse
    pkgs.openssl
    pkgs.postgresql_15

    # To be able to use vim in the terminal
    pkgs.vim
    # utility for search
    pkgs.ack
  ];

  # https://devenv.sh/scripts/
  scripts.hello.exec = "echo hello from $GREET";

  enterShell = ''
    hello
    git --version
  '';

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    # https://devenv.sh/reference/options/#languagesrustversion
    version = "latest";
  };

  # https://devenv.sh/pre-commit-hooks/
  pre-commit.hooks = {
    clippy.enable = true;
    rustfmt.enable = true;
    reuse = {
      enable = true;
      name = "Reuse license headers";
      entry = "${pkgs.reuse}/bin/reuse lint";
      pass_filenames = false;
    };
  };

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";

  # See full reference at https://devenv.sh/reference/options/
}
