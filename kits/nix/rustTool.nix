# Common builder for the repo's Rust binaries.
{
  pkgs,
  root,
  supportedSystems,
}:
{
  pname,
  subdir,
  description,
  runtimeInputs ? [ ],
  extra ? { },
}:
let
  base = {
    inherit pname;
    version = "0.1.0";

    src = pkgs.lib.fileset.toSource {
      inherit root;
      fileset = pkgs.lib.fileset.unions [
        (root + "/Cargo.toml")
        (root + "/Cargo.lock")
        (root + "/welcome/cli")
        (root + "/welcome/gui")
        (root + "/system/services")
        (root + "/kits/nixcore")
        (root + "/kits/localekit")
        (root + "/.cargo")
        (root + "/LICENSE")
        (root + "/kits/neptunia/neptunia.nix")
        (root + "/kits/neptunia/generated/neptunia.slint")
        (root + "/system/templates/home")
      ];
    };
    buildAndTestSubdir = subdir;
    cargoLock.lockFile = root + "/Cargo.lock";

    nativeBuildInputs = [ pkgs.makeWrapper ];

    postInstall = ''
      wrapProgram $out/bin/${pname} \
          --prefix PATH : ${pkgs.lib.makeBinPath runtimeInputs}
    '';

    meta = {
      inherit description;
      mainProgram = pname;
      license = pkgs.lib.licenses.gpl2Only;
      platforms = supportedSystems;
    };
  };
in
pkgs.rustPlatform.buildRustPackage (base // extra)
