{
  pkgs,
  installer,
  theme,
}:

let
  palette = import ../../kits/neptunia/palette.nix { inherit theme; };
  nerd = pkgs.nerd-fonts.jetbrains-mono;
  fontsConf = pkgs.makeFontsConf { fontDirectories = [ nerd ]; };

  paletteArgs = pkgs.lib.concatMap (e: [
    "--${palette.name e.suffix}"
    e.value
  ]) palette.entries;
in
pkgs.writeShellApplication {
  name = "nixdots-installer";
  runtimeInputs = [
    pkgs.kmscon
    nerd
  ];
  text = ''
    installer=${installer}/bin/nixdots-installer
    fonts_conf=${fontsConf}

    if [ "''${TERM:-}" = linux ] \
        && [ -z "''${KEEL_IN_KMSCON:-}" ] \
        && [ -z "''${KEEL_NO_KMSCON:-}" ] \
        && [ -c "$(tty 2>/dev/null || true)" ] \
        && [ -e /dev/dri/renderD128 ]; then
        if FONTCONFIG_FILE="$fonts_conf" KEEL_IN_KMSCON=1 \
            kmscon --vt auto --switchvt --hwaccel \
                --font-name "JetBrainsMono Nerd Font Mono" --font-size 14 \
                ${pkgs.lib.escapeShellArgs paletteArgs} --login -- "$installer" "$@"; then
            exit 0
        fi
        echo "kmscon no pudo iniciar (¿sin DRM?); se usa el modo ASCII." >&2
    fi

    exec "$installer" "$@"
  '';
}
