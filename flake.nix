{
  description = "A simple clipboard manager written in Rust and gtk4.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    #rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          ...
        }:
        {
          _module.args.pkgs = import self.inputs.nixpkgs {
            inherit system;
            #overlays = [
            #  (import inputs.rust-overlay)
            #];
          };
          devShells.default =
            let
              libPath =
                with pkgs;
                lib.makeLibraryPath [
                  libGL
                  libxkbcommon
                  wayland
                  pkg-config
                  libclang
                ];
            in
            pkgs.mkShell {
              inputsFrom = builtins.attrValues self'.packages;
              packages = with pkgs; [
                #(rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
                cargo
                cargo-watch
                rustc
                rust-analyzer
                clippy
              ];
              LD_LIBRARY_PATH = libPath;
              LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            };

          packages =
            let
              lockFile = ./Cargo.lock;
            in
            rec {
              oxipaste = pkgs.callPackage ./nix/default.nix { inherit inputs lockFile; };
              default = oxipaste;
            };
        };
      flake = _: rec {
        nixosModules.home-manager = homeManagerModules.default;
        homeManagerModules = rec {
          oxipaste = import ./nix/hm.nix inputs.self;
          default = oxipaste;
        };
      };
    };
}
