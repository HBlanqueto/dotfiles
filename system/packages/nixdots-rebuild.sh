mode="${1:-sw}"
update=0
case "$mode" in
    sw | switch) mode="switch" ;;
    rb | boot) mode="boot" ;;
    build) mode="build" ;;
    update-sw)
        update=1
        mode="switch"
        ;;
    update-rb)
        update=1
        mode="boot"
        ;;
    *)
        echo "uso: nixdots-rebuild [sw|rb|build|update-sw|update-rb]" >&2
        exit 2
        ;;
esac

repo="${KEEL_REPO:-${NIXDOTS_REPO:-$HOME/dotfiles}}"
if [ "$repo" = "$HOME/dotfiles" ] && [ ! -e "$repo/flake.nix" ] && [ -e "$HOME/nixdots/flake.nix" ]; then
    repo="$HOME/nixdots"
fi

if [ ! -e "$repo/flake.nix" ]; then
    echo "No existe $repo/flake.nix (usa KEEL_REPO para cambiar la ruta canónica)." >&2
    exit 1
fi

cd "$repo"

if [ "$update" = 1 ]; then
    echo "== nix flake update ($repo) =="
    nix flake update --flake "$repo"
fi

ref="$repo#keel"

echo "== nixos-rebuild build ($ref) =="
nixos-rebuild build --impure --flake "$ref"

if [ "$mode" = "build" ]; then
    exit 0
fi

new_users="$(nix eval --impure --json "$repo#nixosConfigurations.keel.config.users.users" --apply '
    users: builtins.mapAttrs (_: u: {
        isNormalUser = u.isNormalUser or false;
    }) users')"

disappearing=()
while IFS= read -r user; do
    [ -z "$user" ] && continue
    if ! jq -e --arg user "$user" '.[$user].isNormalUser == true' >/dev/null <<<"$new_users"; then
        disappearing+=("$user")
    fi
done < <(awk -F: '$3 >= 1000 && $3 < 60000 && $1 !~ /^nixbld/ && $1 != "nobody" { print $1 }' /etc/passwd)

if [ "${#disappearing[@]}" -gt 0 ]; then
    echo "Estos usuarios normales desaparecerían de la configuración: ${disappearing[*]}" >&2
    exit 1
fi

echo "== nixos-rebuild $mode ($ref) =="
sudo nixos-rebuild "$mode" --impure --flake "$ref"
