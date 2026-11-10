{ config, pkgs, ... }:

{
  imports = [
    ./apps.nix
    ../../style.nix
    ./ai.nix
  ];

  home = {
    sessionVariables = {
      BROWSER =
        if pkgs.stdenv.hostPlatform.isx86_64 then
          "${config.programs.brave.package}/bin/brave-origin-beta"
        else
          "${pkgs.firefox}/bin/firefox";
      TERMINAL = "wezterm";
      EDITOR = "code --wait";
      NO_AT_BRIDGE = "1";
      NIXOS_OZONE_WL = "1";
    };
  };

  xdg = {
    enable = true;

    userDirs = {
      enable = true;
      createDirectories = true;

      projects = "${config.home.homeDirectory}/Proyectos";
      publicShare = "${config.home.homeDirectory}/Público";
      templates = "${config.home.homeDirectory}/Plantillas";
      desktop = "${config.home.homeDirectory}/Escritorio";
      documents = "${config.home.homeDirectory}/Documentos";
      music = "${config.home.homeDirectory}/Música";
      pictures = "${config.home.homeDirectory}/Imágenes";
      videos = "${config.home.homeDirectory}/Videos";
      download = "${config.home.homeDirectory}/Descargas";
    };
  };
}
