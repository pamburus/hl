let
  unstable = import (builtins.fetchTarball "https://github.com/NixOS/nixpkgs/archive/nixpkgs-unstable.tar.gz") {};
  pkgs = import <nixpkgs> {
    overlays = [
      (final: prev: {
        cargo = unstable.cargo;
        rustc = unstable.rustc;
      })
    ];
  };
in {
  inherit (pkgs) cargo rustc;
}
