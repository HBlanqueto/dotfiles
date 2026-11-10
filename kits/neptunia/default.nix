let
  colors = import ./neptunia.nix;
in
{
  inherit colors;
  filecolors = import ./filecolors.nix;
}
