let
  colors = import ./neptunia.nix;
in
{
  inherit colors;
  filecolors = import ./filecolors.nix;
  vscode = import ./vscode.nix { inherit colors; };
}
