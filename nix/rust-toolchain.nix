{ lib, pkgs }:
let
  rustToolchainToml = builtins.fromTOML (builtins.readFile ../rust-toolchain.toml);
  toolchainSpec = rustToolchainToml.toolchain;
in
pkgs.rust-bin.stable.${toolchainSpec.channel}.default.override {
  extensions = toolchainSpec.components or [ "rustfmt" "clippy" ];
  targets = toolchainSpec.targets or [ ];
}
