{ theme, ... }:

{
  config.console = {
    earlySetup = true;
    colors = with theme.colors; [
      bg
      c1
      c2
      c3
      c4
      c5
      c6
      c7
      lbg
      c9
      c10
      c11
      c12
      c13
      c14
      c15
    ];
    font = "Lat2-Terminus16";
    useXkbConfig = true;
  };
}
