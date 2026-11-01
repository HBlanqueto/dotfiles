{
  prefs ? { },
  osConfig ? { },
  lib,
  ...
}:

let
  a = prefs.a11y or { };
  on = k: a.${k} or false;
  timeZoneAuto = osConfig.services.automatic-timezoned.enable or false;
  colorScheme = prefs.colorScheme or null;
  accentColor = prefs.accentColor or null;
  gnomeAccents = [
    "blue"
    "teal"
    "green"
    "yellow"
    "orange"
    "red"
    "pink"
    "purple"
    "slate"
  ];
in

{
  programs.wezterm = {
    enable = true;
    extraConfig = builtins.readFile ./wezterm.lua;
  };

  dconf.settings = {
    "org/gnome/desktop/interface" =
      lib.optionalAttrs (on "largeText") { text-scaling-factor = 1.25; }
      // lib.optionalAttrs (on "reduceMotion") { enable-animations = false; }
      // lib.optionalAttrs (colorScheme == "dark") { color-scheme = "prefer-dark"; }
      // lib.optionalAttrs (accentColor != null && builtins.elem accentColor gnomeAccents) {
        accent-color = accentColor;
      };
    "org/gnome/desktop/a11y/interface" = lib.optionalAttrs (on "highContrast") {
      high-contrast = true;
    };
    "org/gnome/desktop/a11y/applications" =
      lib.optionalAttrs (on "screenReader") { screen-reader-enabled = true; }
      // lib.optionalAttrs (on "screenKeyboard") { screen-keyboard-enabled = true; };
    "org/gnome/desktop/datetime" = {
      automatic-timezone = timeZoneAuto;
    };
  };
}
