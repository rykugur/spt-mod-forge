# Shared definition of the development environment.
# Used by both the plain `nix develop` shell (shells/default.nix)
# and the optional devenv configuration (devenv.nix).
#
# This keeps the Rust toolchain, packages, and environment variables
# in a single place so they don't diverge.

{ pkgs }:

let
  # Latest stable Rust with the extensions we care about:
  # - rust-src: for rust-analyzer and IDE jump-to-definition in std
  # - rust-analyzer, clippy, rustfmt: all in one consistent toolchain
  rustToolchain = pkgs.rust-bin.stable.latest.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "clippy"
      "rustfmt"
    ];
  };
in
{
  inherit rustToolchain;

  packages = with pkgs; [
    # The full custom Rust toolchain
    rustToolchain

    # Quality-of-life Cargo tools
    cargo-watch
    cargo-edit
    cargo-deny
    cargo-outdated
    cargo-udeps

    # General dev tools
    just
    jq
    ripgrep

    # Nix formatting (powers `nix fmt`)
    alejandra
  ];

  env = {
    RUST_BACKTRACE = "1";
    RUST_LOG = "info";
    # Helps rust-analyzer and some proc-macro crates find sources
    RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
  };

  # A friendly message shown when entering the shell.
  # (devenv calls this via enterShell; plain mkShell uses shellHook)
  shellHook = ''
    echo "🦀 SPT Mod Forge dev shell"
    echo "   rustc: $(rustc --version | cut -d' ' -f2)"
    echo ""
    echo "Common commands:"
    echo "  cargo build"
    echo "  cargo run --"
    echo "  cargo watch -x build"
    echo "  cargo clippy -- -D warnings"
    echo "  cargo fmt"
  '';
}
