{ lib
, rustPlatform
,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.workspace.package.version;

  src = builtins.path {
    path = ../.;
  };

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "htp-0.4.2" = "sha256-oYLN0aCLIeTST+Ib6OgWqEgu9qyI0n5BDtIUIIThLiQ=";
      "wildflower-0.3.0" = "sha256-vv+ppiCrtEkCWab53eutfjHKrHZj+BEAprV5by8plzE=";
    };
  };

  meta = {
    description = cargoToml.package.description;
    homepage = cargoToml.workspace.package.repository;
    license = lib.licenses.mit;
    mainProgram = cargoToml.package.name;
  };
}
