{}:

let
  theme = import ../../../thm { };
in

with theme.colors;

{
  settings = {
    mgr = {
      ratio = [ 0 1 1 ];

    };

    opener = {
      play_ncmpcpp = [
        {
          run = "mpc clear && mpc add \"file://$@\" && mpc play && ncmpcpp";
          block = true;
        }
      ];
    };

    open = {
      rules = [
        { mime = "audio/*"; use = [ "play_ncmpcpp" ]; }
        { url = "*.mp3";   use = [ "play_ncmpcpp" ]; }
      ];
    };
  };
  
  theme = {
    mgr = {
      #border_style = { fg = "black"; };
      border_symbol = " ";
    };
    indicator = {
      padding = {
        open = "█";
        close = "█";
      };
      current = { 
        reversed = false; 
        fg = "#${bg}";
        bg = "#${fg}";
      };
      parent = {
        reversed = false;
        fg = "#${bg}";
        bg = "#${fg}";
      };
    };
  };
}