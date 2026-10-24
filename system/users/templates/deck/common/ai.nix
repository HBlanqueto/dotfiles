{ pkgs, ... }:

{
  # `uv` provides `uv tool install graphifyy`; installed tools land in
  # ~/.local/bin, which is prepended to PATH below.
  # `nodejs` se necesita para lanzar el MCP de rust-docs con `npx`.
  home = {
    packages = [
      pkgs.uv
      pkgs.nodejs
    ];
    sessionPath = [ "$HOME/.local/bin" ];
  };

  programs.opencode = {
    enable = true;
    package = pkgs.opencode;

    settings = {
      # npm plugins are fetched by opencode at startup, so there is
      # nothing to package here.
      plugin = [ "@dietrichgebert/ponytail" ];

      mcp = {
        nixos = {
          type = "local";
          command = [ "${pkgs.mcp-nixos}/bin/mcp-nixos" ];
        };
        rust-docs = {
          type = "local";
          command = [
            "${pkgs.nodejs}/bin/npx"
            "-y"
            "mcp-rust-docs@latest"
          ];
        };
        gh_grep = {
          type = "remote";
          url = "https://mcp.grep.app";
        };
      };
    };
  };
}
