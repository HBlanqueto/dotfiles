local theme = dofile(os.getenv("HOME") .. "/.config/lua-theme/theme.lua")
local wezterm = require('wezterm')

local font_normal = {
    family = 'UbuntuMono Nerd Font',
    weight = 'Regular'
}

local function font_with_fallback(name, params)
    return wezterm.font_with_fallback({
        name,
        "Twitter Color Emoji",
    }, params)
end

local config = {
    front_end = 'OpenGL',
    enable_wayland = true,
    warn_about_missing_glyphs = true,
    check_for_updates = false,
    bold_brightens_ansi_colors = false,
    default_cursor_style = 'BlinkingUnderline',
    window_close_confirmation = 'NeverPrompt',

    inactive_pane_hsb = {
        saturation = 1.0,
        brightness = 1.0
    },

    window_padding = {
        left = 20,
        right = 20,
        top = 20,
        bottom = 20
    },

    enable_tab_bar = true,
    tab_bar_at_bottom = true,
    hide_tab_bar_if_only_one_tab = true,
    show_tab_index_in_tab_bar = false,

    font_size = 10.2,
    line_height = 1.3,

    font = wezterm.font_with_fallback({
        font_normal,
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
            }, { italic = true })
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
            }, { bold = true, italic = true })
        },
        {
            italic = false,
            intensity = 'Bold',
            font = font_with_fallback({
                family = 'UbuntuMono Nerd Font',
                weight = 'Bold',
            }, { bold = true })
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

    colors = {
        foreground = theme.fg,
        background = theme.bg,
        cursor_bg = theme.fg,
        cursor_fg = theme.fg,
        cursor_border = theme.fg,
        split = theme.lbg,

        ansi = {
            theme.c0, theme.c1, theme.c2, theme.c3, theme.c4, theme.c5,
            theme.c6, theme.c7
        },
        brights = {
            theme.c8, theme.c9, theme.c10, theme.c11, theme.c12, theme.c13,
            theme.c14, theme.c15
        },

        tab_bar = {
            active_tab = {
                bg_color = theme.bg,
                fg_color = theme.c8,
                italic = true
            },
            inactive_tab = {
                bg_color = theme.dbg,
                fg_color = theme.c8
            },
            inactive_tab_hover = {
                bg_color = theme.c0,
                fg_color = theme.bg
            }
        }
    },

    disable_default_key_bindings = true,

    keys = {
        {
            mods = "CTRL",
            key = "h",
            action = wezterm.action {
                SplitHorizontal = {
                    domain = "CurrentPaneDomain"
                }
            }
        },
        {
            mods = "CTRL",
            key = "v",
            action = wezterm.action {
                SplitVertical = {
                    domain = "CurrentPaneDomain"
                }
            }
        },
        {
            mods = "CTRL",
            key = "m",
            action = wezterm.action {
                SpawnTab = "CurrentPaneDomain"
            }
        },
        {
            mods = "CTRL",
            key = "w",
            action = wezterm.action {
                CloseCurrentTab = {
                    confirm = false
                }
            }
        },
        {
            mods = "CTRL",
            key = "Tab",
            action = wezterm.action {
                ActivateTabRelative = 1
            }
        },
        {
            mods = "CTRL|SHIFT",
            key = "Tab",
            action = wezterm.action {
                ActivateTabRelative = -1
            }
        },
        {
            mods = "CTRL|SHIFT",
            key = "v",
            action = wezterm.action {
                PasteFrom = "Clipboard"
            }
        },
        {
            mods = "CTRL|SHIFT",
            key = "c",
            action = wezterm.action {
                CopyTo = "ClipboardAndPrimarySelection"
            }
        },
    },
}

return config