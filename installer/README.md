# install-nixos

Install from a NixOS live ISO:

```sh
nix-shell -p git --run "git clone --depth 1 https://github.com/HBlanqueto/nixdots"
sudo ./nixdots/install.sh
```

Run simulation:

```sh
nix run .#install-nixos
```

Install from the flake:

```sh
sudo nix run .#install-nixos -- --install
```

Options:

- `--install`
- `--repo <path|url>`
