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
  lockFile,
  vulkan-loader,
  wayland-protocols,
  libX11,
  libXrandr,
  libXi,
  libXcursor,
  mesa,
  system,
  ...
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../oxirun/Cargo.toml);
  libPath = lib.makeLibraryPath [
    libGL
    libxkbcommon
    wayland
    pkg-config
    libclang
  ];
  driverIcdPath = "${mesa}/share/vulkan/icd.d";
  icdArch =
    if system == "x86_64-linux"
    then "x86_64"
    else if system == "aarch64-linux"
    then "aarch64"
    else "x86_64";
in
  rustPlatform.buildRustPackage rec {
    pname = cargoToml.package.name;
    inherit (cargoToml.package) version;

    src = ../oxirun/.;

    buildInputs = [
      pkg-config
      libGL
      libxkbcommon
      wayland
      libclang
    ];

    # I legit hate how rust is handled by nix, it's unusable
    cargoLock = {
      inherit lockFile;
      outputHashes = {
        "oxiced-0.5.1" = "sha256-XZfjeMqjCVLG89z6XN/Gkb77bUHaQvzD3yJq6eWXgGo=";
      };
    };

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

    copyLibs = true;
    LD_LIBRARY_PATH = libPath;
    LIBCLANG_PATH = "${libclang.lib}/lib";
    VK_ICD_FILENAMES = "${driverIcdPath}/radeon_icd.${icdArch}.json:${driverIcdPath}/intel_icd.${icdArch}.json";

    postFixup = let
      libPath = lib.makeLibraryPath [
        libGL
        vulkan-loader
        wayland
        wayland-protocols
        libxkbcommon
        libX11
        libXrandr
        libXi
        libXcursor
        libclang
        mesa
      ];
    in ''
      patchelf --set-rpath "${libPath}" "$out/bin/oxirun"
    '';

    meta = with lib; {
      description = "A simple application runner made with Iced";
      homepage = "https://github.com/Xetibo/OxiRun";
      changelog = "https://github.com/Xetibo/OxiRun/releases/tag/${version}";
      license = licenses.gpl3;
      maintainers = with maintainers; [DashieTM];
      mainProgram = "oxirun";
    };
  }
