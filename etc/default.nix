{ config, pkgs, lib, inputs, hostName, username, ... }:

let
  theme = import ../thm { };
in
{
  imports = [
    ./systemd.nix
    ./environment.nix
    ./fonts.nix
    ../hardware-configuration.nix
    ../boot
  ];

  # ==========================================
  # CONFIGURACIÓN DE HOME MANAGER
  # ==========================================
  home-manager = {
    backupFileExtension = "backup";
    sharedModules = [ ];
    # Inyectamos las variables a Home Manager para que tus dotfiles las puedan usar
    extraSpecialArgs = { inherit inputs theme hostName username; };
    # Vinculamos dinámicamente el usuario
    users.${username} = import ../home;
  };

  # ==========================================
  # CONFIGURACIÓN BASE DEL SISTEMA
  # ==========================================
  console = {
    colors = with theme.colors; [ bg c1 c2 c3 c4 c5 c6 c7 ] ++ [ lbg c9 c10 c11 c12 c13 c14 c15 ];
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  networking.hostName = hostName;
  time.timeZone = "America/Merida";
  i18n.defaultLocale = "es_MX.UTF-8";

  programs.fish.enable = true;

  # ==========================================
  # IMPERMANENCE (PERSISTENCIA)
  # ==========================================
  environment.persistence."/persist" = {
    hideMounts = true;
    directories = [
      "/var/log"
      "/var/lib/bluetooth"
      "/var/lib/nixos"
      "/etc/NetworkManager/system-connections"
      "/var/lib/AccountsService"
      "/etc/nixos"
    ];
    files = [
      "/etc/machine-id"
    ];
  };

  # ==========================================
  # USUARIOS
  # ==========================================
  users = { 
    defaultUserShell = pkgs.fish;
    mutableUsers = false;
    users.${username} = {
      isNormalUser = true;
      description = "Humberto B.";
      extraGroups = [ "networkmanager" "wheel" ];
      createHome = true;
      # Tu contraseña encriptada
      hashedPassword = "$6$rvI2ZNaKpc69XPeZ$R6iSUJ3l7iYlFc6eJz4pue1cl51d0H0dBNYkJcTm5BddRohQkdCC7sHmS50UczcPKESV//lw0CpO071roxsB21";
    };
  };

  # ==========================================
  # PAQUETES Y ENTORNO GLOBALES
  # ==========================================
  environment = {
    binsh = "${pkgs.dash}/bin/dash";
    systemPackages = with pkgs; [
      git 
      wget 
      curl 
      polkit_gnome 
      gsettings-desktop-schemas 
      libnotify 
      firefox
      wezterm
    ];
  };

  nixpkgs.config.allowUnfree = true;
  nix.settings.experimental-features = [ "nix-command" "flakes" ];
  documentation.nixos.enable = false;

  system.stateVersion = "26.05";
}