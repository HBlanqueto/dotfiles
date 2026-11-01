# Creates `users.users.<name>` from the external user data.
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
}
