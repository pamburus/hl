{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; }
      {
        systems = [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ];

        perSystem = { self', lib, system, pkgs, config, ... }: {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          packages = {
            default = self'.packages.hl;

            hl = pkgs.callPackage ./nix/package.nix { };

            # Debug build variant
            hl-debug = self'.packages.hl.override {
              buildType = "debug";
            };

            # Static build variant for containers
            hl-static = pkgs.pkgsStatic.callPackage ./nix/package.nix { };

            # Minimal build without shell completions
            hl-minimal = self'.packages.hl.overrideAttrs (old: {
              postInstall = "";
              nativeBuildInputs = [ ];
            });
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ self'.packages.default ];

            packages = with pkgs; [
              # Rust development tools
              rust-analyzer
              cargo-watch
              cargo-edit
              cargo-expand
              cargo-audit
              cargo-outdated

              # Build and development tools
              git
              just
              fd
              ripgrep

              # Documentation and formatting
              mdbook
              taplo

              # Testing and benchmarking
              hyperfine
              valgrind

              # Nix development tools
              nixpkgs-fmt
              nil
              nix-tree
              nix-output-monitor
            ] ++ lib.optionals stdenv.isDarwin [
              # macOS specific tools
              libiconv
              darwin.apple_sdk.frameworks.Security
            ];

            shellHook = ''
              echo "🦀 Rust development environment for hl"
              echo "Available tools:"
              echo "  cargo watch  - Auto-rebuild on file changes"
              echo "  cargo edit   - Manage dependencies"
              echo "  cargo expand - Show macro expansions"
              echo "  cargo audit  - Security audit"
              echo "  just         - Command runner"
              echo "  hyperfine    - Benchmarking"
              echo "  nixpkgs-fmt  - Format Nix files"
              echo "  nil          - Nix language server"
              echo "  nix-tree     - Browse dependency tree"
              echo ""
              echo "Run 'cargo build' to get started!"
            '';

            RUST_SRC_PATH = "${pkgs.rust-bin.stable.latest.default}/lib/rustlib/src/rust/library";
          };

          # Define apps for easy running
          apps = {
            default = self'.apps.hl;

            hl = {
              type = "app";
              program = "${self'.packages.default}/bin/hl";
            };
          };

          checks = {
            # Check Nix formatting
            nixpkgs-fmt = pkgs.runCommand "check-nix-format" { } ''
              ${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt --check ${./.}
              touch $out
            '';

            # Build the package
            package = self'.packages.default;

            # Check that the package can be built with different Rust versions
            package-msrv = pkgs.callPackage ./nix/package.nix {
              rust-bin = pkgs.rust-bin.stable."1.89.0";
            };
          };

          formatter = pkgs.nixpkgs-fmt;
        };
      };
}
