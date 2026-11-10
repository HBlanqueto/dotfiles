# Dev shell for the installer, the wizard and repo tools.
{
  pkgs,
  tablesEnv,
  setup,
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
        setup
        nixfmt
        statix
        deadnix
        nil
      ];
    }
  );
}
