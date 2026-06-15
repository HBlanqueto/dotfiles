{ config, pkgs, inputs, lib, ... }:

let
  theme = import ../theme { };
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
        user.name = "MrHBlanqueto";
      };
    };

    yazi = {
      enable = true;

      plugins = {
        inherit (pkgs.yaziPlugins) 
          full-border
          yatline;
      };

    settings = {
      opener = {
        play_ncmpcpp = [
          {
            run = "mpc clear && mpc add \"file://$@\" && mpc play && ncmpcpp";
            block = true;
          }
        ];
      };

      open = {
        rules = [
          { mime = "audio/*"; use = [ "play_ncmpcpp" ]; }
          { url = "*.mp3";    use = [ "play_ncmpcpp" ]; }
        ];
      };
    };
      
      theme = {
        mgr = {
          border_style = { fg = "black"; };
        };
        indicator = {
          padding = {
          open = "█";   # Dejar vacío elimina el medio círculo izquierdo
          close = "█";  # Dejar vacío elimina el medio círculo derecho
        };
        # Desactivamos 'reversed' para que no use el azul por defecto de tu terminal
        current = { 
          reversed = false; 
          fg = "blue";   # Texto blanco para que resalte
          bg = "black";   # ¡El fondo ROJO que buscamos!
        };
        
        # Opcional: El indicador de la columna izquierda (el "padre")
        parent = {
          reversed = false;
          fg = "blue";
          bg = "black";   # Un rojo más oscuro para la navegación previa
          };
        };
      };

      initLua = ''
        -- Activamos el borde exterior redondeado
        require("full-border"):setup {
          type = ui.Border.ROUNDED,
        }

require("yatline"):setup({
	section_separator = { open = "", close = "" },
	part_separator = { open = "", close = "" },
	inverse_separator = { open = "", close = "" },

	padding = { inner = 1, outer = 1 },

	style_a = {
		bg = "white",
		fg = "black",
		bg_mode = {
			normal = "white",
			select = "brightyellow",
			un_set = "brightred",
		},
	},
	style_b = { bg = "brightblack", fg = "brightwhite" },
	style_c = { bg = "black", fg = "brightwhite" },

	permissions_t_fg = "green",
	permissions_r_fg = "yellow",
	permissions_w_fg = "red",
	permissions_x_fg = "cyan",
	permissions_s_fg = "white",

	tab_width = 20,

	selected = { icon = "󰻭", fg = "yellow" },
	copied = { icon = "", fg = "green" },
	cut = { icon = "", fg = "red" },

	files = { icon = "", fg = "blue" },
	filtereds = { icon = "", fg = "magenta" },

	total = { icon = "󰮍", fg = "yellow" },
	success = { icon = "", fg = "green" },
	failed = { icon = "", fg = "red" },

	show_background = true,

	display_header_line = false,
	display_status_line = true,

	component_positions = { "tab", "status" },

	status_line = {
		left = {
			section_a = {
				{ type = "string", name = "tab_mode" },
			},
			section_b = {
				{ type = "string", name = "hovered_size" },
			},
			section_c = {
				{ type = "string", name = "hovered_path" },
				{ type = "coloreds", name = "count" },
			},
		},
		right = {
			section_a = {
				{ type = "string", name = "cursor_position" },
			},
			section_b = {
				{ type = "string", name = "cursor_percentage" },
			},
			section_c = {
				{ type = "string", name = "hovered_file_extension", params = { true } },
				{ type = "coloreds", name = "permissions" },
			},
		},
	},
})
      '';
    };

    ncmpcpp = {
      enable = true;
      package = pkgs.ncmpcpp.override { visualizerSupport = true; };
      settings = import ./config/ncmpcpp.nix;
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
        set -g fish_color_command --bold green""'';

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

      settings = import ./config/mopidy.nix { };
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
    allowUnsupportedSystem = true;
  };
}