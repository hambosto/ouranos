{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.ouranos;
  tomlFormat = pkgs.formats.toml { };
in
{
  options.services.ouranos = {
    enable = lib.mkEnableOption "Whether to enable ouranos";

    package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      description = "The ouranos package to use.";
    };

    settings = lib.mkOption {
      inherit (tomlFormat) type;
      default = { };
      example = lib.literalExpression ''
        {
          image.path = "~/wallpapers/wallpaper.png";
        }
      '';
      description = ''
        Configuration written to
        {file}`$XDG_CONFIG_HOME/ouranos/config.toml`.
        See <https://github.com/hambosto/ouranos>
        for the full list of options.
      '';
    };
  };

  config = lib.mkIf cfg.enable {

    xdg.configFile = {
      "ouranos/config.toml" = lib.mkIf (cfg.settings != { }) {
        source = tomlFormat.generate "ouranos-config.toml" cfg.settings;
      };
    };

    systemd.user.services.ouranos = lib.mkIf (cfg.package != null) {
      Install.WantedBy = [ config.wayland.systemd.target ];

      Service = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
      };

      Unit = {
        After = [ config.wayland.systemd.target ];
        ConditionEnvironment = "WAYLAND_DISPLAY";
        Description = "Set the sky of your desktop — a Wayland wallpaper daemon with animated transitions.";
        Documentation = "https://github.com/hambosto/ouranos";
        PartOf = [ config.wayland.systemd.target ];
        X-Restart-Triggers = lib.mkIf (cfg.settings != { }) [
          "${config.xdg.configFile."ouranos/config.toml".source}"
        ];
      };
    };
  };
}
