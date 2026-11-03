{
  impure ? (builtins ? currentSystem),
}:
let
  keelRoot = builtins.getEnv "KEEL_ROOT";
  legacyRoot = builtins.getEnv "NIXDOTS_ROOT";
  root = if keelRoot != "" then keelRoot else legacyRoot;

  persistKeel = "${root}/persist/keel";
  persistLegacy = "${root}/persist/nixdots";

  persist =
    if impure && !builtins.pathExists persistKeel && builtins.pathExists persistLegacy then
      persistLegacy
    else
      persistKeel;

  readRaw =
    file:
    let
      path = "${persist}/${file}";
    in
    if impure && builtins.pathExists path then builtins.readFile path else null;

  readData =
    base:
    let
      toml = readRaw "${base}.toml";
    in
    if toml != null then
      builtins.fromTOML toml
    else
      let
        json = readRaw "${base}.json";
      in
      if json != null then builtins.fromJSON json else { };

  machine = readData "machine";

  usersDir = "${persist}/users";
  nameFrom = ext: file: builtins.head (builtins.match "([^.]+)\\.${ext}" file);
  userNames =
    let
      entries =
        if impure && builtins.pathExists usersDir then
          builtins.attrNames (builtins.readDir usersDir)
        else
          [ ];
      isSeed = n: builtins.match ".*\\.prefs\\.(toml|json)" n != null;
      pick = ext: builtins.filter (n: builtins.match ".*\\.${ext}" n != null && !(isSeed n)) entries;
    in
    builtins.attrNames (
      builtins.listToAttrs (
        map (n: {
          name = n;
          value = null;
        }) (map (nameFrom "toml") (pick "toml") ++ map (nameFrom "json") (pick "json"))
      )
    );
  usersFromDir = builtins.listToAttrs (
    map (
      name:
      let
        value = readData "users/${name}";
      in
      {
        inherit name;
        value = value // {
          username = value.username or name;
        };
      }
    ) userNames
  );
  mismatched = builtins.filter (name: usersFromDir.${name}.username != name) userNames;

  legacy = readData "user";
  legacyUsers =
    if legacy ? username then
      {
        ${legacy.username} = {
          inherit (legacy) username;
          fullName = legacy.fullName or legacy.userdescription or "";
          admin = true;
        };
      }
    else
      { };

  users = if usersFromDir != { } then usersFromDir else legacyUsers;
  hasUsers = users != { };

  settings = import ./defaults.nix // machine;

  facterFile = "${persist}/facter.json";
in
{
  inherit
    impure
    root
    machine
    settings
    users
    hasUsers
    ;

  facterReport = if impure && builtins.pathExists facterFile then facterFile else null;

  usersValid = mismatched == [ ];
  usersError =
    "En ${usersDir} cada cuenta debe llamarse <username>.toml y su campo "
    + "`username` coincidir con el nombre del archivo. Revisa: "
    + builtins.concatStringsSep ", " mismatched;

  firstBoot = impure && !hasUsers;

  exists = path: impure && builtins.pathExists (root + path);

  prefsSeed =
    name: builtins.substring (builtins.stringLength root) (-1) "${usersDir}/${name}.prefs.toml";
}
