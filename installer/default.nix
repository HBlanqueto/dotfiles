{ pkgs, theme, tablesEnv }:

let
  root = ../.;
  runtimeInputs = with pkgs; [
      disko
      btrfs-progs
      dosfstools
      util-linux
      cryptsetup
      systemd
      mkpasswd
      sbctl
      nixos-facter
      less
      git
      nix
    ];

  installer = pkgs.rustPlatform.buildRustPackage (
    {
      pname = "nixdots-installer";
      version = "0.1.0";

      src = pkgs.lib.fileset.toSource {
        inherit root;
        fileset = pkgs.lib.fileset.unions [
          ./Cargo.toml
          ./Cargo.lock
          ./src
          ./assets
          (root + "/kits/nixcore")
          (root + "/kits/localekit")
          (root + "/kits/neptunia/neptunia.nix")
          (root + "/.cargo")
          (root + "/LICENSE")
        ];
      };
      buildAndTestSubdir = "installer";
      cargoRoot = "installer";
      cargoLock.lockFile = ./Cargo.lock;

      nativeBuildInputs = [ pkgs.makeWrapper ];

      postInstall = ''
        wrapProgram $out/bin/nixdots-installer \
            --prefix PATH : ${pkgs.lib.makeBinPath runtimeInputs}
      '';

      meta = {
        description = "Text-mode installer for the Keel NixOS configuration";
        mainProgram = "nixdots-installer";
        license = pkgs.lib.licenses.gpl2Only;
      };
    }
    // tablesEnv pkgs
  );
in
{
  inherit installer;

  launcher = pkgs.callPackage ./nix/launcher.nix {
    inherit installer theme;
  };

  kmscon-palette = pkgs.callPackage ./nix/kmscon-palette.nix { inherit theme; };
}
