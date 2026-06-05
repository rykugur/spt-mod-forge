{
  description = "SPT Mod Forge - Quick TUI mod manager for SPTarkov (Rust + Ratatui)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";

    # Provides the exact Rust toolchain we want (with rust-analyzer, clippy, rust-src, etc.)
    # via shells/common.nix
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, rust-overlay, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem =
        { config, pkgs, system, ... }:
        {
          # Apply rust-overlay globally for this system so that shells/common.nix
          # (and anything else) gets the enhanced packages.
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              (import rust-overlay)
            ];
            config.allowUnfree = true;
          };

          # The one and only default development shell.
          #
          # `nix develop` (and direnv + nix-direnv) will give you the full
          # Rust + tooling environment.
          #
          # No `devenv` CLI is required. No extra tools beyond Nix.
          devShells.default = import ./shells/default.nix { inherit pkgs; };

          # `nix fmt` support
          formatter = pkgs.alejandra;
        };
    };
}
