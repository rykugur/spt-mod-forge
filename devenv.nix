{ pkgs, lib, config, inputs, ... }:

let
  # Ensure we have the rust-overlay applied, whether this module is
  # loaded by the plain flake (already overlaid) or directly by the
  # `devenv` CLI (via the input declared in devenv.yaml).
  pkgsWithRustOverlay =
    if inputs ? rust-overlay then
      pkgs.extend (import inputs.rust-overlay)
    else
      pkgs;

  # Re-use the exact same Rust toolchain + package list + env vars
  # that the plain `nix develop` shell uses.
  # This way `devenv shell` gives a (as close as possible) consistent experience.
  common = import ./shells/common.nix { pkgs = pkgsWithRustOverlay; };
in
{
  # Required so devenv works reliably under flakes (pure eval).
  # See https://devenv.sh/guides/using-with-flakes/
  devenv.root = toString ./.;

  # We pull the packages and environment from the shared common definition.
  # This avoids duplicating the Rust toolchain, cargo-* tools, etc.
  packages = common.packages;

  env = common.env;

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    # Use the exact same toolchain we defined in shells/common.nix
    toolchain = common.rustToolchain;
  };

  # https://devenv.sh/basics/
  enterShell = ''
    ${common.shellHook}

    # Extra devenv-specific hints (optional)
    echo ""
    echo "Devenv extras available: devenv up, devenv test, tasks, etc."
  '';

  # You can add devenv-only features here without affecting plain `nix develop` users:
  #
  # processes.myapp.exec = "cargo run --";
  # git-hooks.hooks.rustfmt.enable = true;
  # tasks."project:setup".exec = "cargo fetch";
  #
  # See https://devenv.sh/reference/options/ for everything.
}
