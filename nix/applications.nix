{
  rustPlatform,
  pkg-config,
  libGL,
  libxkbcommon,
  wayland,
  libclang,
  cargo,
  cargo-watch,
  rustc,
  rust-analyzer,
  clippy,
  lib,
  ...
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../plugins/applications/Cargo.toml);
  lockFile = ../plugins/applications/Cargo.lock;
in
  rustPlatform.buildRustPackage rec {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;

    src = ../plugins/applications/.;

    buildInputs = [
      pkg-config
      libGL
      libxkbcommon
      wayland
      libclang
    ];

    nativeBuildInputs = [
      pkg-config
      wayland
      cargo
      cargo-watch
      rustc
      rust-analyzer
      clippy
      libGL
      libxkbcommon
      libclang
    ];

    cargoLock = {
      inherit lockFile;
    };

    copyLibs = true;

    meta = with lib; {
      description = "Application plugin for OxiRun";
      homepage = "https://github.com/Xetibo/OxiRun";
      changelog = "https://github.com/Xetibo/OxiRun/releases/tag/${version}";
      license = licenses.gpl3;
      maintainers = with maintainers; [DashieTM];
      mainProgram = "oxirun-applications";
    };
  }
