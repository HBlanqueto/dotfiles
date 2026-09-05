# NOTE: Modifying these variables will affect the global configuration,
# including ./etc/desktop/global.nix and other system modules.

{
    system = "x86_64-linux";

    hostName = "nixos";
    timeZone = "America/Merida";
    defaultLocale = "es_MX.UTF-8";

    username = "humbe";
    userdescription = "H. Blanqueto";
    hashedpassword = "$6$rvI2ZNaKpc69XPeZ$R6iSUJ3l7iYlFc6eJz4pue1cl51d0H0dBNYkJcTm5BddRohQkdCC7sHmS50UczcPKESV//lw0CpO071roxsB21";

    # Git user details. This modifies home-manager data in ./home/bin.nix
    gitName = "H. Blanqueto";
    gitEmail = "mc4w6wmkrv@privaterelay.appleid.com";
}