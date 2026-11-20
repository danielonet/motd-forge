#!/bin/sh
# Build/refresh a signed flat apt repository.
#
#   build-apt-repo.sh SITE_DIR DEB_DIR
#
# Copies *.deb from DEB_DIR into SITE_DIR (older versions already there are kept),
# regenerates Packages/Release, signs with the default secret key in the current
# GPG home, and writes the public key as motd-forge.gpg (binary) and motd-forge.asc.
# Set GPG_PASSPHRASE if the key has one. Needs: dpkg-dev, gpg, gzip, sha256sum.
set -eu

site=${1:?usage: build-apt-repo.sh SITE_DIR DEB_DIR}
debs=${2:?usage: build-apt-repo.sh SITE_DIR DEB_DIR}

mkdir -p "$site"
cp "$debs"/*.deb "$site"/
cd "$site"

dpkg-scanpackages --multiversion . /dev/null > Packages 2>/dev/null
gzip -9kf Packages

hashes() {
    for f in Packages Packages.gz; do
        printf ' %s %s %s\n' "$(sha256sum "$f" | cut -d' ' -f1)" "$(wc -c < "$f")" "$f"
    done
}
{
    echo "Origin: motd-forge"
    echo "Label: motd-forge"
    echo "Suite: stable"
    echo "Codename: stable"
    echo "Architectures: all"
    echo "Date: $(date -Ru)"
    echo "Description: motd-forge apt repository"
    echo "SHA256:"
    hashes
} > Release

key=$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr/ {print $10; exit}')
[ -n "$key" ] || { echo "no GPG secret key available" >&2; exit 1; }

sign() {
    if [ -n "${GPG_PASSPHRASE:-}" ]; then
        gpg --batch --yes --pinentry-mode loopback --passphrase "$GPG_PASSPHRASE" -u "$key" "$@"
    else
        gpg --batch --yes --pinentry-mode loopback --passphrase '' -u "$key" "$@"
    fi
}
sign --clearsign -o InRelease Release
sign --armor --detach-sign -o Release.gpg Release

gpg --batch --yes --export "$key" > motd-forge.gpg
gpg --batch --yes --armor --export "$key" > motd-forge.asc
touch .nojekyll

cat > index.html <<'HTML'
<!doctype html><meta charset="utf-8"><title>motd-forge apt repository</title>
<h1>motd-forge apt repository</h1>
<pre>
curl -fsSL https://danielonet.github.io/motd-forge/motd-forge.gpg | sudo tee /usr/share/keyrings/motd-forge.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/motd-forge.gpg] https://danielonet.github.io/motd-forge ./" | sudo tee /etc/apt/sources.list.d/motd-forge.list
sudo apt update &amp;&amp; sudo apt install motd-forge
</pre>
HTML
