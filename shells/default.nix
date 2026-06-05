# The default development shell.
#
# This is a plain pkgs.mkShell so that anyone with Nix (no devenv CLI required)
# can get a fully working environment just by running:
#
#   nix develop
#
# All the important bits (Rust toolchain, packages, env vars) live in ./common.nix
# so they stay in sync with the optional devenv setup.

{ pkgs }:

let
  common = import ./common.nix { inherit pkgs; };
in
pkgs.mkShell {
  packages = common.packages;

  env = common.env;

  shellHook = common.shellHook;
}
