# Home-manager modules exported as `homeModules`.
let
  theme = import ../../../kits/neptunia;

  gitOptions = { lib, ... }: {
    imports = [ (lib.mkAliasOptionModule [ "nixdots" "git" ] [ "system" "git" ]) ];

    options.system.git = {
      name = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "Nombre de git del usuario.";
      };
      email = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "Correo de git del usuario.";
      };
    };
  };
in
rec {
  deck = { ... }: {
    imports = [
      gitOptions
      ./deck
    ];
    _module.args.theme = theme;
  };

  raft = { ... }: {
    imports = [
      gitOptions
      ./raft
    ];
    _module.args.theme = theme;
  };

  nixdots = deck;
  minimal = raft;
}
