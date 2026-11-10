# Dev shell for the installer and repo tools.
{
  pkgs,
  tablesEnv,
}:
{
  default = pkgs.mkShell (
    (tablesEnv pkgs)
    // {
      packages = with pkgs; [
        cargo
        rustc
        rustfmt
        rust-analyzer
        clippy
        watchexec
        gcc
        clang
        lld
        pkg-config
        nixfmt
        statix
        deadnix
        nil
      ];
    }
  );
}
