# nixdots

Install:

```shell
nix-shell -p git --run "git clone --depth 1 https://github.com/HBlanqueto/nixdots"
sudo ./nixdots/install.sh
```

Manual install:

```shell
nixos-install --root /mnt --flake ~/nixdots#nixdots --impure
```
