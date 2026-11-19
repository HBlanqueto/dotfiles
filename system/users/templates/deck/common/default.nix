{ config, pkgs, ... }:

{
  imports = [
    ./apps.nix
    ../../style.nix
    ./ai.nix
  ];

  home = {
    sessionVariables = {
      BROWSER = "${pkgs.firefox}/bin/firefox";
      TERMINAL = "wezterm";
      EDITOR = "code --wait";
      NO_AT_BRIDGE = "1";
      NIXOS_OZONE_WL = "1";
    };
  };

  # GNOME starts apps from the systemd user manager, which never reads hm-session-vars.sh.
  systemd.user.sessionVariables = config.home.sessionVariables;

  programs.git = {
    enable = true;
    settings.credential.helper = "gh auth git-credential";
  };

  xdg = {
    enable = true;

    mimeApps = {
      enable = true;
      defaultApplications = {
        "text/html" = "firefox.desktop";
        "x-scheme-handler/http" = "firefox.desktop";
        "x-scheme-handler/https" = "firefox.desktop";
        "x-scheme-handler/about" = "firefox.desktop";
        "x-scheme-handler/unknown" = "firefox.desktop";
      };
    };

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
