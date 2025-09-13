{
  lib,
  stdenv,
  makeRustPlatform,
  rust-bin,
  installShellFiles,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  toolchain = rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
  rustPlatform = makeRustPlatform {
    cargo = toolchain;
    rustc = toolchain;
  };
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.workspace.package.version;

  src = builtins.path {
    path = ../.;
  };

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = import ./cargo-hashes.nix;
  };

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd hl \
      --bash <($out/bin/hl --shell-completions bash) \
      --fish <($out/bin/hl --shell-completions fish) \
      --zsh <($out/bin/hl --shell-completions zsh)
    $out/bin/hl --man-page >hl.1
    installManPage hl.1
  '';

  doCheck = false;

  meta = {
    description = cargoToml.package.description;
    homepage = cargoToml.workspace.package.repository;
    license = lib.licenses.mit;
    changelog = "${cargoToml.workspace.package.repository}/releases";
    maintainers = [
      {
        name = "Pavel Ivanov";
        github = "pamburus";
        email = "mr.pavel.ivanov@gmail.com";
      }
    ];
    platforms = lib.platforms.unix ++ lib.platforms.windows;
    mainProgram = cargoToml.package.name;
    categories = cargoToml.package.categories or [ ];
    keywords = cargoToml.package.keywords or [ ];
  };
}
