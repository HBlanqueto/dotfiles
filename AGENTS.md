# AGENTS.md

This repo keeps documentation minimal: English only, installation instructions only, no elaborate explanations.

## Rules

- Work directly. Don't ask for verification unless asked.
- Don't change `stateVersion`. Don't add heavy dependencies without justification.
- Don't modify anything outside the repo. Never run `nixos-rebuild switch`, `home-manager switch`, `nixdots-setup apply`, or disk tools on the host.
- Test with `nix build`, `nix flake check`, or `./dev.sh sim`.
- Push directly to `master`. Run `git pull --ff-only` before pushing. Never force-push or rewrite published history.
- Keep commits small and conventional: `feat(...)`, `fix(...)`, `docs(...)`, etc.
- Keep `deadnix` and `statix` clean. Delete dead code; don't comment it out.
- Documentation (this file included) is in English and stays minimal.
