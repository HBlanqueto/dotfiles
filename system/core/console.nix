{
  config,
  lib,
  pkgs,
  theme,
  ...
}:

let
  palette = import ../../kits/neptunia/palette.nix { inherit theme; };

  xnuFont = "${pkgs.xnu-console-font}/share/consolefonts/xnu-console.psf";
in
{
  options.keel.console.hwRender = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = "Aceleración 3D (DRM) en kmscon. Falso en UTM/virtio, donde no aporta.";
  };

  config.console = {
    earlySetup = true;
    colors = palette.consoleColors;
    font = xnuFont;
    useXkbConfig = true;
  };

  config.services.kmscon = {
    enable = true;
    useXkbConfig = true;
    config = {
      hwaccel = config.keel.console.hwRender;
      font-engine = "psf";
      font-name = xnuFont;
      font-size = 16;
    }
    // builtins.listToAttrs (
      map (e: {
        name = palette.name e.suffix;
        inherit (e) value;
      }) palette.entries
    );
  };
}
