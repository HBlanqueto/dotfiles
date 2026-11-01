{
  lib,
  stdenv,
  fetchurl,
  zstd,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "consoleet-darwin";
  version = "20211008";

  src = fetchurl {
    url = "https://inai.de/files/consoleet/consoleet-darwin-${finalAttrs.version}.tar.zst";
    hash = "sha256-OStyp7CfiXnxdhn4rYDQnySy+o1lLCRTm+ExTaJ+lz0=";
  };

  nativeBuildInputs = [ zstd ];

  installPhase = ''
    runHook preInstall

    install -Dm644 consoleet_xnu.otf $out/share/fonts/opentype/consoleet_xnu.otf
    install -Dm644 consoleet_xnuv.otf $out/share/fonts/opentype/consoleet_xnuv.otf

    runHook postInstall
  '';

  meta = {
    description = "macOS console font (XNU) from the Consoleet Darwin project, as OTF";
    homepage = "https://inai.de/projects/consoleet/darwin";
    license = lib.licenses.apple-psl20;
    platforms = lib.platforms.all;
  };
})
