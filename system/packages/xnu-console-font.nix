{
  lib,
  stdenv,
  fetchurl,
  python3,
}:

stdenv.mkDerivation (_finalAttrs: {
  pname = "xnu-console-font";
  version = "12377.121.6";

  src = fetchurl {
    url = "https://raw.githubusercontent.com/apple-oss-distributions/xnu/ac9718fb1af618d5ce8678d0dc6e8a58f252216f/osfmk/console/iso_font.c";
    hash = "sha256-SOY0BIXq6ywqKDkn83SlEdXgFt5p5u+HN381+rc3BQA=";
  };

  nativeBuildInputs = [ python3 ];

  dontUnpack = true;

  buildPhase = ''
    runHook preBuild

    python3 ${./xnu-console-font.py} "$src"

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    install -Dm644 xnu-console.psf $out/share/consolefonts/xnu-console.psf

    runHook postInstall
  '';

  meta = {
    description = "8x16 console font embedded in XNU, the macOS kernel (osfmk/console/iso_font.c), as PSF2";
    homepage = "https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/console/iso_font.c";
    license = lib.licenses.apple-psl20;
    platforms = lib.platforms.all;
  };
})
