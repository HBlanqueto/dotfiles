# Console palette: the 16 ANSI colors.
{ theme }:
let
  inherit (theme) colors;

  digits = {
    "0" = 0;
    "1" = 1;
    "2" = 2;
    "3" = 3;
    "4" = 4;
    "5" = 5;
    "6" = 6;
    "7" = 7;
    "8" = 8;
    "9" = 9;
    "a" = 10;
    "b" = 11;
    "c" = 12;
    "d" = 13;
    "e" = 14;
    "f" = 15;
  };
  byte = s: digits.${builtins.substring 0 1 s} * 16 + digits.${builtins.substring 1 1 s};
  rgb =
    c:
    "${toString (byte (builtins.substring 0 2 c))},${toString (byte (builtins.substring 2 2 c))},${
      toString (byte (builtins.substring 4 2 c))
    }";

  black = "000000";

  consoleColors = [
    black
    colors.c1
    colors.c2
    colors.c3
    colors.c4
    colors.c5
    colors.c6
    colors.c7
    colors.lbg
    colors.c9
    colors.c10
    colors.c11
    colors.c12
    colors.c13
    colors.c14
    colors.c15
  ];

  entries = [
    {
      suffix = "";
      value = "custom";
    }
    {
      suffix = "black";
      value = rgb colors.c0;
    }
    {
      suffix = "red";
      value = rgb colors.c1;
    }
    {
      suffix = "green";
      value = rgb colors.c2;
    }
    {
      suffix = "yellow";
      value = rgb colors.c3;
    }
    {
      suffix = "blue";
      value = rgb colors.c4;
    }
    {
      suffix = "magenta";
      value = rgb colors.c5;
    }
    {
      suffix = "cyan";
      value = rgb colors.c6;
    }
    {
      suffix = "light-grey";
      value = rgb colors.c7;
    }
    {
      suffix = "dark-grey";
      value = rgb colors.c8;
    }
    {
      suffix = "light-red";
      value = rgb colors.c9;
    }
    {
      suffix = "light-green";
      value = rgb colors.c10;
    }
    {
      suffix = "light-yellow";
      value = rgb colors.c11;
    }
    {
      suffix = "light-blue";
      value = rgb colors.c12;
    }
    {
      suffix = "light-magenta";
      value = rgb colors.c13;
    }
    {
      suffix = "light-cyan";
      value = rgb colors.c14;
    }
    {
      suffix = "white";
      value = rgb colors.c15;
    }
    {
      suffix = "foreground";
      value = rgb colors.fg;
    }
    {
      suffix = "background";
      value = rgb black;
    }
  ];

  name = suffix: if suffix == "" then "palette" else "palette-${suffix}";
in
{
  inherit
    name
    entries
    rgb
    black
    consoleColors
    ;
}
