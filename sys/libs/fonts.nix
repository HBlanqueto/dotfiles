{ pkgs, ... }: {

  fonts = {
    fontDir.enable = true;
    enableGhostscriptFonts = true;

    packages = with pkgs; [
      cantarell-fonts
      noto-fonts-cjk-sans
      noto-fonts-cjk-serif
      noto-fonts-color-emoji

      nerd-fonts.ubuntu-mono

      victor-mono

      corefonts
      vista-fonts 
      twitter-color-emoji
    ];

    fontconfig = {
      defaultFonts = {
        monospace = [ "UbuntuMono Nerd Font" "Victor Mono" ];
        sansSerif = [ "Cantarell" "Noto Sans CJK SC" ];
        serif     = [ "Noto Serif CJK SC" ];
        emoji     = [ "Twitter Color Emoji" "Noto Color Emoji" ];
      };
    };
  };

}