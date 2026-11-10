{
  writeShellApplication,
  nix,
  git,
  jq,
  gawk,
  coreutils,
}:

writeShellApplication {
  name = "rebuild-nixos";
  runtimeInputs = [
    nix
    git
    jq
    gawk
    coreutils
  ];
  text = builtins.readFile ./rebuild-nixos.sh;

  meta = {
    description = "Safe nixos-rebuild wrapper";
    mainProgram = "rebuild-nixos";
  };
}
