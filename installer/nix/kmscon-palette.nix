# Renders the neptunia palette as kmscon `--palette*` arguments.
{ pkgs, theme }:

let
  palette = import ../../kits/neptunia/palette.nix { inherit theme; };
in
pkgs.writeText "kmscon-palette" (
  pkgs.lib.concatMapStrings (e: "--${palette.name e.suffix}\n${e.value}\n") palette.entries
)
