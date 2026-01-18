FROM nixos/nix:2.33.1

# Configure Nix to enable flakes and nix-command
ENV NIX_CONFIG="experimental-features = nix-command flakes"

# Install `just`, `ruff`, and `nixfmt` into the root user's nix profile
RUN nix profile add \
    nixpkgs#just \
    nixpkgs#nixfmt-classic \
    nixpkgs#ruff  \
    && nix-collect-garbage -d

# Make sure the profile is on PATH for non-login shells
ENV PATH=/root/.nix-profile/bin:$PATH
