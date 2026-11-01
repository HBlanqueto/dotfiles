{ inputs, ... }:

{
  imports = [ inputs.malaysia.nixosModules.default ];

  programs.malaysia.enable = true;
}
