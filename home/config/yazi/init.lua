local colors = dofile(os.getenv("HOME") .. "/.config/lua-theme/theme.lua")

local sep_01 = {
    type = "coloreds",
    custom = true,
    name = { { "", colors.c0 } }
}

local sep_02 = {
    type = "coloreds",
    custom = true,
    name = { { "", colors.c0 } }
}

require("yatline"):setup({
    section_separator = { open = "", close = "" },
    part_separator = { open = "", close = "" },
    inverse_separator = { open = "", close = "" },

    padding = { inner = 1, outer = 1 },

    style_a = {
        bg = colors.bg,
        fg = colors.fg,
        bg_mode = {
            normal = colors.bg,
            select = colors.bg,
            un_set = colors.bg,
        },
    },
    style_b = { bg = colors.bg, fg = colors.fg },
    style_c = { bg = colors.bg, fg = colors.fg },

    permissions_t_fg = colors.c2,
    permissions_r_fg = colors.c3,
    permissions_w_fg = colors.c1,
    permissions_x_fg = colors.c6,
    permissions_s_fg = colors.c7,

    tab_width = 20,

    selected = { icon = "󰻭", fg = colors.c3 },
    copied = { icon = "", fg = colors.c2 },
    cut = { icon = "", fg = colors.c1 },

    files = { icon = "", fg = colors.c4 },
    filtereds = { icon = "", fg = colors.c5 },

    total = { icon = "󰮍", fg = colors.c3 },
    success = { icon = "", fg = colors.c2 },
    failed = { icon = "", fg = colors.c1 },

    show_background = false,
    display_header_line = true,
    display_status_line = true,

    component_positions = { "header", "tab", "status" },

    header_line = {
        left = {
            section_a = {
                { type = "line", custom = false, name = "tabs", params = { "left" } },
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
            section_c = {},
        }
    }
})