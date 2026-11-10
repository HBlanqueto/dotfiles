# Home Manager template

Apply:

```sh
home-manager switch -b backup --flake ~/.config/home-manager#default
```

If `~/.config/home-manager` is a git repo, track `flake.nix`, `home.nix`, and `prefs.toml`.
