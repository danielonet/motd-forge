#!/bin/sh
# One-line installer for the motd-forge apt repository (Debian/Ubuntu):
#
#   curl -fsSL https://danielonet.github.io/motd-forge/install.sh | sudo sh
#
# Adds the signing key and apt source, then installs motd-forge.
# Read it first if you like: it only does what you see below.
# MOTD_FORGE_DRY_RUN=1 prints the steps without changing anything.
set -eu

BASE=${MOTD_FORGE_URL:-https://danielonet.github.io/motd-forge}
KEYRING=/usr/share/keyrings/motd-forge.gpg
LIST=/etc/apt/sources.list.d/motd-forge.list

run() {
    if [ -n "${MOTD_FORGE_DRY_RUN:-}" ]; then echo "+ $*"; else "$@"; fi
}

if [ -z "${MOTD_FORGE_DRY_RUN:-}" ]; then
    [ "$(id -u)" -eq 0 ] || { echo "Run as root: curl -fsSL $BASE/install.sh | sudo sh" >&2; exit 1; }
    command -v apt-get >/dev/null 2>&1 || { echo "This installer needs apt (Debian/Ubuntu)." >&2; exit 1; }
    command -v curl >/dev/null 2>&1 || { echo "curl is required: apt-get install curl" >&2; exit 1; }
fi

echo "Adding the motd-forge apt repository ($BASE)"
if [ -n "${MOTD_FORGE_DRY_RUN:-}" ]; then
    echo "+ curl -fsSL $BASE/motd-forge.gpg -o $KEYRING"
    echo "+ write $LIST: deb [signed-by=$KEYRING] $BASE ./"
else
    curl -fsSL "$BASE/motd-forge.gpg" -o "$KEYRING"
    chmod 644 "$KEYRING"
    echo "deb [signed-by=$KEYRING] $BASE ./" > "$LIST"
fi

run apt-get update
run apt-get install -y motd-forge

echo "Done. Try: motd-forge preview fortress --demo"
