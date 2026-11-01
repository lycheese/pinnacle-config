#!/usr/bin/env bash
# Installs the Helix editor from its official release tarball into ~/.local (no sudo), for
# distros without a recent package (Ubuntu / Pop!_OS 24.04). Fedora: `dnf install helix`.
#
#   ~/.local/lib/helix-<version>/   binary + runtime (grammars, themes)
#   ~/.local/bin/hx                 symlink; Helix finds its runtime next to the real binary
#
# Re-running with the same version does nothing. Override the version with HELIX_VERSION.
set -euo pipefail

version="${HELIX_VERSION:-25.07.1}"
arch="$(uname -m)"
dest="$HOME/.local/lib/helix-$version"
bin="$HOME/.local/bin/hx"

if [[ -x "$dest/hx" ]]; then
    echo "Helix $version already installed in $dest"
else
    url="https://github.com/helix-editor/helix/releases/download/$version/helix-$version-$arch-linux.tar.xz"
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    echo "Downloading $url"
    curl --proto '=https' --tlsv1.2 -fL "$url" -o "$tmp/helix.tar.xz"
    mkdir -p "$tmp/x"
    tar -xJf "$tmp/helix.tar.xz" -C "$tmp/x" --strip-components=1
    mkdir -p "$(dirname "$dest")"
    mv "$tmp/x" "$dest"
fi

mkdir -p "$(dirname "$bin")"
ln -sfn "$dest/hx" "$bin"
"$bin" --version

case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) echo "Add ~/.local/bin to your PATH so \`hx\` is found." ;;
esac
