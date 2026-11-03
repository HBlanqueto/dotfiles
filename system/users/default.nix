{ lib, data, ... }:

let
  mkUser = name: u: {
    isNormalUser = true;
    description = u.fullName or name;
    extraGroups = [
      "networkmanager"
      "video"
    ]
    ++ lib.optional (u.admin or false) "wheel";
    createHome = true;
    hashedPasswordFile = "/persist/secrets/${name}.hash";
  };
in
{
  users.users = lib.mapAttrs mkUser data.users;

  # El asistente escribe la semilla antes de que exista la cuenta; sin esto home-init no puede leerla.
  systemd.tmpfiles.rules = map (name: "z ${data.prefsSeed name} 0600 ${name} - - -") (
    builtins.attrNames data.users
  );
}
