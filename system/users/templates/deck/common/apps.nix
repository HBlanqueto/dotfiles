{
  config,
  pkgs,
  lib,
  prefs ? { },
  theme,
  ...
}:

let
  starshipPlain = pkgs.runCommand "starship-plain-text-symbols.toml" { } ''
    ${pkgs.starship}/bin/starship preset plain-text-symbols -o $out
  '';
in

{
  xdg = {
    configFile = {
      "eza/theme.yml".source = (pkgs.formats.yaml { }).generate "eza-theme.yml" {
        filekinds = {
          directory = {
            foreground = theme.filecolors.directory;
            is_bold = true;
          };
          executable = {
            foreground = theme.filecolors.executable;
            is_bold = true;
          };
          symlink = {
            foreground = theme.filecolors.symlink;
          };
        };
        links = {
          normal = {
            foreground = theme.filecolors.symlink;
          };
          multi_link = {
            foreground = theme.filecolors.symlink;
          };
          broken = {
            foreground = theme.filecolors.orphan;
          };
        };
        broken_symlink = {
          foreground = theme.filecolors.orphan;
        };
      };
    };
  };

  home = {
    file = {
      ".config/fastfetch/logo.txt".source = ./config/fastfetch/logo.txt;
      ".config/fastfetch/config.jsonc".source = ./config/fastfetch/config.jsonc;

      "Pictures/Screenshots/.gitkeep".text = "";
    };

    packages =
      with pkgs;
      [
        fastfetch
        eza
        luajit
        lua51Packages.luacheck
        mpc
        ffmpeg
        poppler-utils
        gh
        python3

        cava
        firefox
        vscode
        nautilus

        karere

        imagemagick
        quickemu
      ]
      ++ lib.optionals pkgs.stdenv.hostPlatform.isx86_64 [
        onlyoffice-desktopeditors
      ]
      ++ lib.optional (prefs.a11y.screenReader or false) pkgs.orca;
  };

  programs = {
    brave = lib.mkIf pkgs.stdenv.hostPlatform.isx86_64 {
      enable = true;
      package = pkgs.brave-origin-beta.overrideAttrs (old: {
        preFixup =
          (old.preFixup or "") + "\ngappsWrapperArgs+=(\"--add-flags\" \"--ozone-platform-hint=auto\")\n";
      });
    };

    git = {
      enable = true;
      settings.user = {
        inherit (config.keel.git) name email;
      };
    };

    ssh = {
      enable = true;
      enableDefaultConfig = false;
      settings = {
        "*" = {
          ForwardAgent = false;
          AddKeysToAgent = "no";
          Compression = false;
          ServerAliveInterval = 0;
          ServerAliveCountMax = 3;
          HashKnownHosts = false;
          UserKnownHostsFile = "~/.ssh/known_hosts";
          ControlMaster = "no";
          ControlPath = "~/.ssh/master-%r@%n:%p";
          ControlPersist = "no";
        };
      };
    };

    mpv = {
      enable = true;
      scripts = [ pkgs.mpvScripts.mpris ];
    };

    fish = {
      enable = true;
      interactiveShellInit = ''
        set -g fish_greeting
        set -g fish_color_command --bold green

        if test "$XDG_SESSION_TYPE" = tty
            set -gx STARSHIP_CONFIG ${starshipPlain}
        end
        set -g eza_icons --icons=auto
        alias ls "eza --color=auto $eza_icons"
        alias l "ls -l"
        alias la "ls -a"
        alias lla "ls -la"
        alias lt "ls --tree"
      '';
      shellAliases = {
        home-update = "nix flake update --flake $HOME/.config/home-manager keel && home-manager switch -b backup --flake $HOME/.config/home-manager#default";

        g = "git";
        c = "clear";
      };
    };

    starship = {
      enable = true;
    };
  };
}
