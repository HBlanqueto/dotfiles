{theme}:

with theme.colors;

''
  local wezterm = require('wezterm')

  local font_normal = {  
    family = 'UbuntuMono Nerd Font', 
    weight = 'Regular' 
  }

  local font_fallback_ubuntu = { 
    family = 'UbuntuMono Nerd Font'
  }

  local function font_with_fallback(name, params)
    return wezterm.font_with_fallback({
      name,
      "Twitter Color Emoji",
    }, params)
  end

  config = {
    front_end = 'OpenGL',

    enable_wayland = true,

    warn_about_missing_glyphs = true,

    check_for_updates = false,

    bold_brightens_ansi_colors = true,

    default_cursor_style = 'BlinkingUnderline',

    window_close_confirmation = 'NeverPrompt',

    inactive_pane_hsb = { 
      saturation = 1.0, 
      brightness = 1.0 
    },

    window_padding = { 
      left = 40, 
      right = 40, 
      top = 40, 
      bottom = 40 
    },

    enable_tab_bar = true,
    tab_bar_at_bottom = true,
    hide_tab_bar_if_only_one_tab = true,
    show_tab_index_in_tab_bar = false,

    font_size = 10.2,
    line_height = 1.3,

    font = wezterm.font_with_fallback({
      font_normal,
      font_fallback_ubuntu,
      "Twitter Color Emoji",
    }),
    
    font_rules = {
      { 
        italic = true, 
        intensity = 'Normal',
        font = wezterm.font_with_fallback({ 
          { 
            family = 'Victor Mono', 
            style = 'Italic', 
            scale = 0.8 
          }, 
            "Twitter Color Emoji"
        }, 
              { italic = true })
      },

      { 
        italic = true, 
        intensity = 'Bold', 
        font = wezterm.font_with_fallback({ 
          { 
          family = 'Victor Mono', 
          style = 'Italic', 
          weight = 'Bold', 
          scale = 0.8 
        }, 
          "Twitter Color Emoji"
        }, 
            { bold = true, italic = true })
      },

      { 
        italic = false,
        intensity = 'Bold', 
        font = font_with_fallback({ 
          family = 'UbuntuMono Nerd Font', 
          weight = 'Bold',
        }, 
            { bold = true }) 
      },

      { 
        italic = false,
        intensity = 'Half', 
        font = font_with_fallback({ 
          family = 'UbuntuMono Nerd Font', 
          weight = 'DemiBold', 
        }) 
      },
    },

    -- Pretty Colors
      bold_brightens_ansi_colors = false,

      colors = {
          foreground = "#${fg}",
          background = "#${bg}",
          cursor_bg = "#${fg}",
          cursor_fg = "#${fg}",
          cursor_border = "#${fg}",
          split = "#${lbg}",

          ansi = {
              "#${c0}", "#${c1}", "#${c2}", "#${c3}", "#${c4}", "#${c5}",
              "#${c6}", "#${c7}"
          },
          brights = {
              "#${c8}", "#${c9}", "#${c10}", "#${c11}", "#${c12}", "#${c13}",
              "#${c14}", "#${c15}"
          },

          tab_bar = {
              active_tab = {
                  bg_color = "#${bg}",
                  fg_color = "#${c8}",
                  italic = true
              },
              inactive_tab = {bg_color = "#${dbg}", fg_color = "#${c8}"},
              inactive_tab_hover = {bg_color = "#${c0}", fg_color = "#${bg}"}
          }

      },

    disable_default_key_bindings = true,

    keys = {
      { mods = "CTRL", key = "o", action = wezterm.action { 
        SplitHorizontal = { 
          domain = "CurrentPaneDomain" 
          } 
        } 
      },

      { mods = "CTRL", key = "p", action = wezterm.action { 
        SplitVertical = { 
          domain = "CurrentPaneDomain" 
          } 
        } 
      },

      { mods = "CTRL", key = "m", action = wezterm.action { 
        SpawnTab = "CurrentPaneDomain" 
        } 
      },

      { mods = "CTRL", key = "w", action = wezterm.action { 
        CloseCurrentTab = { 
          confirm = false 
          } 
        } 
      },

      { mods = "CTRL", key = "Tab", action = wezterm.action { 
        ActivateTabRelative = 1 
        } 
      },

      { mods = "CTRL|SHIFT", key = "Tab", action = wezterm.action { 
        ActivateTabRelative = -1 
        } 
      },

      { mods = "CTRL|SHIFT", key = "v", action = wezterm.action { 
        PasteFrom = "Clipboard" 
        } 
      },

      { mods = "CTRL|SHIFT", key = "c", action = wezterm.action { 
        CopyTo = "ClipboardAndPrimarySelection" 
        } 
      },
    },
  }

  return config
''