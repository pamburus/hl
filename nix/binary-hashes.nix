# Binary release hashes for hl
#
# This file is automatically updated by GitHub Actions when new releases are published.
# Each release version contains SHA256 hashes for all platform-specific binary assets.
#
# Hash format: Nix SRI format (sha256-base64hash)
# Asset naming: hl-{platform}-{arch}[-{libc}].{ext}
#
# DO NOT EDIT MANUALLY - changes will be overwritten by automation

{
  "0.32.0" = {
    "hl-linux-x86_64-musl.tar.gz" = "sha256-NvatH7J5/xjd/vmhE6CKe6BIC17b4+AQIS2OOFDgUxA=";
    "hl-linux-arm64-musl.tar.gz" = "sha256-lbmJF9ECksCL/A0QMysTyQjOwVLi4hyOB0uFzpYA1Cs=";
    "hl-macos-x86_64.tar.gz" = "sha256-kcf18Us6KKAzdhTv98ashNTBJRDq1dBEJztJMoOUnso=";
    "hl-macos-arm64.tar.gz" = "sha256-kcf18Us6KKAzdhTv98ashNTBJRDq1dBEJztJMoOUnso=";
  };
  "$VERSION" = {
  "hl-linux-x86_64-musl.tar.gz" = "$LINUX_X64";
  "hl-linux-arm64-musl.tar.gz" = "$LINUX_ARM64";
  "hl-macos-x86_64.tar.gz" = "$MACOS_X64";
  "hl-macos-arm64.tar.gz" = "$MACOS_ARM64";
};
  # New versions will be automatically added here by GitHub Actions
}
