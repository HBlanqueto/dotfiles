{ config, pkgs, inputs, lib, ... }:

let
  theme = import ../thm { };
in

{
  home = {
    username = "humbe";
    homeDirectory = "/home/humbe";

    sessionVariables = {
      BROWSER = "${pkgs.firefox}/bin/firefox";
      TERMINAL = "wezterm";

      SDL_VIDEODRIVER = "wayland";
      QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
      QT_QPA_PLATFORM="wayland;xcb";
      QT_QPA_PLATFORMTHEME="gnome"; # or "qt5ct"
      CLUTTER_BACKEND = "wayland";
      NO_AT_BRIDGE = "1";
    };


    packages = with pkgs; [
      fastfetch
      eza
      mpc
      ffmpeg

      python3

      vscode
      wezterm
      nautilus
      telegram-desktop
    ];

    stateVersion = "26.05";
  };

  programs = {
    brave = {
      enable = true;
      # package = inputs.brave-origin.packages.${pkgs.stdenv.hostPlatform.system}.default;
    };
    
    git = {
      enable = true;
      settings = {
        user.email = "mc4w6wmkrv@privaterelay.appleid.com";
        user.name = "H. Blanqueto";
      };
    };

    yazi = {
      enable = true;

      plugins = {
        inherit (pkgs.yaziPlugins) 
          #full-border
          yatline;
      };
    } // import ./config/yazi.nix {};

    ncmpcpp = {
      enable = true;
      package = pkgs.ncmpcpp.override { visualizerSupport = true; };
      settings = import ./config/ncmpcpp.nix;
    };

    rmpc = {
      enable = true;
      config = import ./config/rmpc.nix;
    };

    bat = {
      enable = true;
      config = {
        pager = "never";
        style = "full";
        theme = "base16";
      };
    };

    fish = {
      enable = true;

      # Desactivate Greeting.
      interactiveShellInit = ''
        set -g fish_greeting 
        set -g fish_color_command --bold green""
      '';

      shellAliases = {
        delgen = "sudo nix-collect-garbage --delete-older-than 1d && sudo nix-store --gc && sudo nix-env --profile /nix/var/nix/profiles/system --delete-generations old";
        nix-update = "sudo nixos-rebuild switch";
        flake-update-rb = "sudo nixos-rebuild boot --flake .#NixOS --impure";
        flake-update-sw = "sudo nixos-rebuild switch --flake .#NixOS --impure";

        g = "git";
        c = "clear";

        ls = "eza --color=auto --icons";
        l = "ls -l";
        la = "ls -a";
        lla = "ls -la";
        lt = "ls --tree";
      };
    };

    starship = {
      enable = true;
      settings = import ./config/starship.nix;
    };

    home-manager = {
      enable = true;
    };
  };

  services = {
    mopidy = {
      enable = true;

      extensionPackages = with pkgs; [
        mopidy-local
        mopidy-mpd
      ];

      settings = import ./config/mopidy.nix {};
    };
  };

  fonts.fontconfig.enable = true;

  xdg = {
    enable = true;

    configFile = {
      "wezterm/wezterm.lua".text = import ./config/wezterm.nix { inherit theme; };
    };

    userDirs = {
      enable = true;
      desktop = "${config.home.homeDirectory}/Escritorio";
      documents = "${config.home.homeDirectory}/Documentos";
      music = "${config.home.homeDirectory}/Música";
      pictures = "${config.home.homeDirectory}/Imágenes";
      videos = "${config.home.homeDirectory}/Videos";
      download = "${config.home.homeDirectory}/Descargas";
    };
  };

  nixpkgs.config = {
    allowUnfree = true;
  };
}