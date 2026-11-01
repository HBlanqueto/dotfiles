{ pkgs, ... }:

{
  home = {
    packages = [ pkgs.uv ];
    sessionPath = [ "$HOME/.local/bin" ];
  };

  programs.opencode = {
    enable = true;
    package = pkgs.opencode;

    settings = {
      plugin = [ "@dietrichgebert/ponytail" ];

      mcp = {
        "mcp-nixos" = {
          type = "local";
          command = [ "${pkgs.mcp-nixos}/bin/mcp-nixos" ];
        };
      };
    };
  };
}
