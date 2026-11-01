{ pkgs, theme, ... }:

let
  themeKeys = [
    "dbg"
    "lbg"
    "fg"
    "bg"
  ]
  ++ map (n: "c${toString n}") (pkgs.lib.range 0 15);
  themeLua =
    "return {\n"
    + pkgs.lib.concatMapStrings (k: "    ${k} = \"#${theme.colors.${k}}\",\n") themeKeys
    + "}\n";
in

{
  xdg = {
    configFile = {
      "lua-theme/theme.lua".text = themeLua;
    };
  };

  fonts = {
    fontconfig = {
      enable = true;
    };
  };
}
