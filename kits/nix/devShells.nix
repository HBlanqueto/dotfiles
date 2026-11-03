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
        setup
        nixfmt
        statix
        deadnix
        nil
      ];
    }
  );
}
