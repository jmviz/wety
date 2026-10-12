#!/usr/bin/env bash
# Install the pinned rclone (arm64 .deb from rclone.org, checksum-verified). Used by
# the deploy workflow and bootstrap.sh. Ubuntu's own package (1.60) is too old for
# R2: every upload logs a 501 NotImplemented and passes only on retry.
#
#   sudo ./deploy/install-rclone.sh
set -euo pipefail

version=1.75.1
sha256=773f3a76615f91f7d4654183a537afddce3343c8d99ac1d74984f060f2ade2d9 # rclone-v$version-linux-arm64.deb

if [ "$(rclone version 2>/dev/null | sed -n '1s/^rclone v//p')" = "$version" ]; then
    echo "rclone $version already installed"
    exit 0
fi
[ "$(dpkg --print-architecture)" = arm64 ] || {
    echo "install-rclone.sh: pinned for arm64, this is $(dpkg --print-architecture)" >&2
    exit 1
}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
deb=$tmp/rclone.deb
curl -fsSL -o "$deb" "https://downloads.rclone.org/v$version/rclone-v$version-linux-arm64.deb"
echo "$sha256  $deb" | sha256sum -c --quiet -
dpkg -i "$deb"
rclone version | head -n 1
