{
  stdenvNoCC,
  fetchurl,
  lib,
}:

let
  rev = "dc5a3e6fcc2e16ad476b7be3c3c17c2273b260ea";
  base = "https://raw.githubusercontent.com/shaunsingh/SFMono-Nerd-Font-Ligaturized/${rev}";

  weights = {
    Regular = "sha256-MMOToFyVHMGAL/pMEoB0LexyTKDx38MSecoXcrEeuys=";
    RegularItalic = "sha256-b9nQO8Ri0SV+WzHAQXlNN9VfxhI32YKOar4kv2MV4Zg=";
    Bold = "sha256-yZpKMKDEXh/EbCZvexVCq3JjQmWumLSP1vJhncjlJLc=";
    BoldItalic = "sha256-P8j7Ab2MXUPG23l72SqkPAAjodGXm1M5NAcR4LJC3kE=";
    Semibold = "sha256-W5CpfC0k0ErrIDAza6VmF6voWK13/dQO051ULNkxeZs=";
  };

  fonts = lib.mapAttrs (
    name: hash:
    fetchurl {
      url = "${base}/LigaSFMonoNerdFont-${name}.otf";
      inherit hash;
    }
  ) weights;
in
stdenvNoCC.mkDerivation {
  pname = "sf-mono-liga-bin";
  version = "dev";

  dontUnpack = true;
  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    mkdir -p $out/share/fonts/opentype
    ${lib.concatStringsSep "\n" (
      lib.mapAttrsToList (
        name: f: "cp ${f} $out/share/fonts/opentype/LigaSFMonoNerdFont-${name}.otf"
      ) fonts
    )}
  '';

  meta = {
    description = "Apple SFMono font with ligatures and Nerd Font patches";
  };
}
