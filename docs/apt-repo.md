# Publishing the apt repository

## Releasing

A release is created automatically. Bump `version` in `pyproject.toml` and the top entry of
`debian/changelog` (e.g. `0.1.1` and `0.1.1-1`) and merge to `main`: if there is no GitHub release
for that version yet, CI creates the tag `v<version>`, attaches the `.deb`, and updates the apt
repository. Pushing a `v*` tag by hand works too.

Every release builds the `.deb`, attaches it to the GitHub release and publishes a signed
flat apt repository to the `gh-pages` branch (served by GitHub Pages).

## One-time setup

1. Create a signing key (no passphrase is simplest; if you set one, store it as `APT_GPG_PASSPHRASE`):

   ```
   gpg --quick-gen-key "motd-forge apt <you@example.com>" rsa4096 sign never
   gpg --armor --export-secret-keys "motd-forge apt" > motd-forge-secret.asc
   ```

2. In the repo: Settings, Secrets and variables, Actions, add secret `APT_GPG_PRIVATE_KEY` with the
   contents of `motd-forge-secret.asc` (and optionally `APT_GPG_PASSPHRASE`). Then delete the local
   secret file or keep it in a password manager.
3. Release (merge to `main` with an unreleased version, or push a `v*` tag). Once the `apt-repo` job has created the `gh-pages` branch, enable
   Settings, Pages, Source: Deploy from a branch, `gh-pages` / root.

## Installing from the repository

One line: `curl -fsSL https://danielonet.github.io/motd-forge/install.sh | sudo sh`
(the script is published from `scripts/install.sh`; `MOTD_FORGE_DRY_RUN=1` prints the steps only).
Or by hand:

```
curl -fsSL https://danielonet.github.io/motd-forge/motd-forge.gpg | sudo tee /usr/share/keyrings/motd-forge.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/motd-forge.gpg] https://danielonet.github.io/motd-forge ./" | sudo tee /etc/apt/sources.list.d/motd-forge.list
sudo apt update && sudo apt install motd-forge
```

Later releases arrive through `apt upgrade`. Older versions stay in the repository.

## Notes

- Built on Debian 13; the package needs Python 3.11 or newer.
- Local test of the script: `scripts/build-apt-repo.sh SITE_DIR DEB_DIR` with a throwaway GPG key.
- Each release also carries a stable-name copy, `motd-forge_all.deb`, so
  `https://github.com/danielonet/motd-forge/releases/latest/download/motd-forge_all.deb` always
  works. The apt repository only gets the versioned file.
