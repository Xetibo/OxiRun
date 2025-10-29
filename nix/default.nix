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
        "cryoglyph-0.1.0" = "sha256-Jc+rhzd5BIT7aYBtIfsBFFKkGChdEYhDHdYGiv4KE+c=";
        "dpi-0.1.1" = "sha256-pQn1lCFSJMkjUfHoggEzMHnm5k+Chnzi5JEDjahnjUA=";
        "iced-0.14.0-dev" = "sha256-xPTbJw/Zmk2+x/Ojc5Px0LVW8sSxCKggrg3N/O7BBBo=";
        "iced_exdevtools-0.14.0-dev" = "sha256-EJ9G4EERnJhJ0vhGny8oZC77E2lNzF1y1iCvHhrYsNo=";
        "oxiced-0.5.1" = "sha256-HpG1SaY+ejwc23N3LFq8gW3A4uqlcwRbumstHrR8XxM=";
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
