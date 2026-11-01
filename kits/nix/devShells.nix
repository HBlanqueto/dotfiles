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
        slint-viewer
        slint-lsp
        fontconfig
        freetype
        libxkbcommon
        libglvnd
        wayland
        libx11
        setup
        nixfmt
        statix
        deadnix
        nil
      ];
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
        with pkgs;
        [
          wayland
          libxkbcommon
          libglvnd
          fontconfig
          freetype
          libx11
        ]
      );
    }
  );
}
