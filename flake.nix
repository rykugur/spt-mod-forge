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
        { pkgs, system, ... }:
        let
          # Use the same custom Rust toolchain (via rust-overlay) for building the
          # release binary as we use in the dev shell. This ensures consistency.
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
              "rustfmt"
            ];
          };

          buildPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          spt-mod-forge = buildPlatform.buildRustPackage {
            # NOTE: When rusqlite(bundled), zip, etc are added we rely on rustPlatform
            # vendoring. If build fails in CI/nix, add:
            # nativeBuildInputs = [ pkgs.pkg-config ];
            # buildInputs = [ ];  # empty for pure-rust + bundled paths

            pname = "spt-mod-forge";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            # No native dependencies needed for this basic Ratatui + Crossterm TUI.
            # When we add more (e.g. for archive handling later) we can extend
            # buildInputs / nativeBuildInputs here.
          };
        in
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

          packages = {
            inherit spt-mod-forge;
            default = spt-mod-forge;
          };

          apps = {
            spt-mod-forge = {
              type = "app";
              program = "${spt-mod-forge}/bin/spt-mod-forge";
            };
            default = {
              type = "app";
              program = "${spt-mod-forge}/bin/spt-mod-forge";
            };
          };

          # `nix fmt` support
          formatter = pkgs.alejandra;
        };
    };
}
