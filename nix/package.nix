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

  meta = with lib; {
    description = cargoToml.package.description;
    longDescription = ''
      hl is a fast log viewer and processor that converts JSON and logfmt logs
      into a human-readable format. It supports filtering, highlighting, and
      various output formats for better log analysis and debugging.
    '';
    homepage = cargoToml.workspace.package.repository;
    changelog = "${cargoToml.workspace.package.repository}/releases";
    license = licenses.mit;
    maintainers = [
      {
        name = "Pavel Ivanov";
        github = "pamburus";
        email = "mr.pavel.ivanov@gmail.com";
      }
    ];
    platforms = platforms.unix ++ platforms.windows;
    mainProgram = cargoToml.package.name;
    categories = cargoToml.package.categories or [ ];
    keywords = cargoToml.package.keywords or [ ];
  };
}
