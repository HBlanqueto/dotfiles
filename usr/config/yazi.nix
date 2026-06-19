{}:

let
  theme = import ../../thm { };
in

with theme.colors;

{
  settings = {
    mgr = {
      ratio = [ 0 1 1 ];

    };

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
        { url = "*.mp3";   use = [ "play_ncmpcpp" ]; }
      ];
    };
  };
  
  theme = {
    mgr = {
      #border_style = { fg = "black"; };
      border_symbol = " ";
    };
    indicator = {
      padding = {
        open = "█";
        close = "█";
      };
      current = { 
        reversed = false; 
        fg = "#${bg}";
        bg = "#${fg}";
      };
      parent = {
        reversed = false;
        fg = "#${bg}";
        bg = "#${fg}";
      };
    };
  };

  initLua = ''
    --require("full-border"):setup {
    --  type = ui.Border.ROUNDED,
    --}

    local sep_01 = { 
        type = "coloreds", 
        custom = true, 
        name = { {"", "#${c0}" } }
        }

    local sep_02 = { 
        type = "coloreds", 
        custom = true, 
        name = { {"", "#${c0}" } }
        }

require("yatline"):setup({
  section_separator = { open = "​", close = "​" },
	part_separator = { open = "​", close = "​" },
	inverse_separator = { open = "​", close = "​" },

    padding = { inner = 1, outer = 1 },

    style_a = {
        bg = "#${bg}",
        fg = "#${fg}",
        bg_mode = {
            normal = "#${bg}",
            select = "#${bg}",
            un_set = "#${bg}",
        },
    },
    style_b = { bg = "#${bg}", fg = "#${fg}" },
    style_c = { bg = "#${bg}", fg = "#${fg}" },

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

    show_background = false,
    display_header_line = true,
    display_status_line = true,

    component_positions = { "header", "tab", "status" },

    header_line = {
        left = {
            section_a = {
                { type = "line", custom = false, name = "tabs", params = {"left"} },
            },
            section_b = {},
            section_c = {},
        },
        right = {
            section_a = {},
            section_b = {},
            section_c = {},
        }
    },

    status_line = {
        left = {
            section_a = { 
                { type = "string", name = "hovered_file_extension", params = { true } }, 
            },
            section_b = { 
                { type = "string", name = "hovered_size" }, 
            },
            section_c = { 
                { type = "string", name = "hovered_path" },
            },
        },
        right = {
            section_a = { 
                { type = "string", name = "cursor_position" }, 
            },
            section_b = {             
                { type = "coloreds", name = "count" },
                { type = "coloreds", name = "permissions" },
            },
            section_c = { },
        }
    }
})
  '';
}