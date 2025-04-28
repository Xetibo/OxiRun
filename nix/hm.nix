self: {
  config,
  pkgs,
  lib,
  ...
}: let
  cfg = config.programs.oxirun;
  defaultPackage = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
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
  };
  config = lib.mkIf cfg.enable {
    home.packages = lib.optional (cfg.package != null) cfg.package;
  };
}
