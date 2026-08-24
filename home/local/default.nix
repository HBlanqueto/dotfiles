{ config, pkgs, lib, inputs, ... }:

{
    home.packages = with pkgs; [
        fastfetch
        eza
        mpc
        ffmpeg
        efibootmgr
        python3
        trezor-suite
        vscode
        nautilus

        karere
        telegram-desktop
        bitwarden-desktop
        
        morewaita-icon-theme

        onlyoffice-desktopeditors
    ];

    home.stateVersion = "26.05";

programs = {
        brave = {
            enable = true;

            package = inputs.brave-previews.packages.${pkgs.stdenv.hostPlatform.system}.brave-origin-beta;
        };

        git = {
            enable = true;
            settings = {
                user = {
                    email = "mc4w6wmkrv@privaterelay.appleid.com";
                    name = "H. Blanqueto";
                };
            };
        };

        yazi = {
            enable = true;
            enableFishIntegration = true;
            plugins = {
                inherit (pkgs.yaziPlugins) yatline;
            };
            initLua = builtins.readFile ../config/yazi/init.lua;
            
            settings = (import ../config/yazi {}).settings;
            theme = (import ../config/yazi {}).theme;
        };

        ncmpcpp = {
            enable = true;
            package = pkgs.ncmpcpp.override { visualizerSupport = true; };
            settings = import ../config/ncmpcpp.nix;
        };

        bat = {
            enable = true;
            config = {
                pager = "never";
                style = "full";
                theme = "base16";
            };
        };

        wezterm = {
            enable = true;
            extraConfig = builtins.readFile ../config/wezterm.lua;
        };

        fish = {
            enable = true;
            interactiveShellInit = ''
                set -g fish_greeting 
                set -g fish_color_command --bold green
            '';
            shellAliases = {
                delgen = "sudo nix-collect-garbage --delete-older-than 1d && sudo nix-store --gc && sudo nix-env --profile /nix/var/nix/profiles/system --delete-generations old";
                nix-update = "sudo nixos-rebuild switch";
                flake-update-rb = "sudo nixos-rebuild boot --flake .#nixos --impure";
                flake-update-sw = "sudo nixos-rebuild switch --flake .#nixos --impure";

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
            settings = import ../config/starship.nix;
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
            settings = import ../config/mopidy.nix {};
        };
    };
}