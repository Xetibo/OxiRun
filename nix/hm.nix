self: {
  config,
  pkgs,
  lib,
  ...
}: let
  cfg = config.programs.oxirun;
  defaultPackage = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
  oxirun-applications = self.packages.${pkgs.stdenv.hostPlatform.system}.oxirun-applications;
in {
  meta.maintainers = with lib.maintainers; [DashieTM];
  options.programs.oxirun = with lib; {
    enable = mkEnableOption "oxirun";

    package = mkOption {
      type = with types; nullOr package;
      default = defaultPackage;
      defaultText = lib.literalExpression ''
        oxirun.packages.''${pkgs.stdenv.hostPlatform.system}.default
      '';
      description = mdDoc ''
        Package to run
      '';
    };

    config = {
      plugins = mkOption {
        type = with types; listOf package;
        default = [oxirun-applications];
        example = [];
        description = mdDoc ''
          List of plugins to use, represented as a list of packages.
        '';
      };

      plugin_config = mkOption {
        type = with types; attrs;
        default = {};
        description = mdDoc ''
          Toml values passed to the configuration for plugins to use.
        '';
      };
    };
  };
  config = let
    fetchedPlugins =
      if cfg.config.plugins == []
      then []
      else
        builtins.map
        (entry:
          if lib.types.package.check entry
          then "lib${lib.replaceStrings ["-"] ["_"] entry.pname}.so"
          else "")
        cfg.config.plugins;
  in
    lib.mkIf
    cfg.enable
    {
      home.packages = lib.optional (cfg.package != null) cfg.package ++ cfg.config.plugins;
      home.file = builtins.listToAttrs (builtins.map
        (pkg: {
          name = ".config/oxirun/plugins/lib${lib.replaceStrings ["-"] ["_"] pkg.pname}.so";
          value = {
            source = "${pkg}/lib/lib${lib.replaceStrings ["-"] ["_"] pkg.pname}.so";
          };
        })
        cfg.config.plugins);

      xdg.configFile."oxirun/config.toml".source =
        (pkgs.formats.toml {}).generate "oxirun"
        (lib.recursiveUpdate
          {
            plugins = fetchedPlugins;
          }
          cfg.config.plugin_config);
    };
}
