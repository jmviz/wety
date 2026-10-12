#!/usr/bin/env bash
# Prepare an Ubuntu 24.04 host to run wety. Idempotent: rerun it after changing
# anything in deploy/ other than graph.json. See deploy/README.md.
#
#   sudo ./deploy/bootstrap.sh [deploy-public-key-file]
#
# The optional key file holds the public half of the GitHub Actions deploy key;
# it may only run wety-deploy.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
deploy_key=${1:-}

[ "$(id -u)" = 0 ] || {
    echo "run as root: sudo $0 $*" >&2
    exit 1
}
# shellcheck source=/dev/null
. /etc/os-release
if [ "${ID:-}" != ubuntu ] || [ "${VERSION_ID:-}" != 24.04 ]; then
    echo "warning: written for Ubuntu 24.04, this is ${PRETTY_NAME:-unknown}" >&2
fi

echo "== packages"
if [ ! -f /etc/apt/sources.list.d/cloudflared.list ]; then
    install -d -m 755 /usr/share/keyrings
    curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg -o /usr/share/keyrings/cloudflare-main.gpg
    echo "deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared ${VERSION_CODENAME:-noble} main" \
        >/etc/apt/sources.list.d/cloudflared.list
fi
apt-get update -q
DEBIAN_FRONTEND=noninteractive apt-get install -y -q cloudflared curl jq rclone unattended-upgrades

echo "== users"
id wety >/dev/null 2>&1 || useradd --system --home-dir /srv/wety --shell /usr/sbin/nologin wety
id deploy >/dev/null 2>&1 || useradd --create-home --shell /bin/bash deploy

echo "== files"
install -d -m 755 /srv/wety /srv/wety/releases /srv/wety/graphs
install -d -m 755 /etc/wety
install -m 755 "$here/wety-deploy" /usr/local/sbin/wety-deploy
unit_changed=false
if ! cmp -s "$here/wety.service" /etc/systemd/system/wety.service; then
    install -m 644 "$here/wety.service" /etc/systemd/system/wety.service
    unit_changed=true
fi
if [ ! -f /etc/wety/r2.env ]; then
    install -m 600 /dev/null /etc/wety/r2.env
    cat >/etc/wety/r2.env <<'EOF'
# Read-only R2 token for the wety-deploy bucket (wety-deploy-host, deploy/README.md).
RCLONE_CONFIG_R2_TYPE=s3
RCLONE_CONFIG_R2_PROVIDER=Cloudflare
RCLONE_CONFIG_R2_ACCESS_KEY_ID=
RCLONE_CONFIG_R2_SECRET_ACCESS_KEY=
RCLONE_CONFIG_R2_ENDPOINT=https://ACCOUNT_ID.r2.cloudflarestorage.com
RCLONE_CONFIG_R2_NO_CHECK_BUCKET=true
WETY_R2_BUCKET=wety-deploy
EOF
    echo "created /etc/wety/r2.env: fill in the R2 credentials"
fi

echo "== deploy access"
sudoers=/etc/sudoers.d/wety-deploy
echo 'deploy ALL=(root) NOPASSWD: /usr/local/sbin/wety-deploy' >"$sudoers.tmp"
chmod 440 "$sudoers.tmp"
visudo -cqf "$sudoers.tmp"
mv "$sudoers.tmp" "$sudoers"
if [ -n "$deploy_key" ]; then
    install -d -m 700 -o deploy -g deploy /home/deploy/.ssh
    # The key can only run wety-deploy; sshd passes the requested command
    # (e.g. "deploy <commit>") in SSH_ORIGINAL_COMMAND and wety-deploy validates it.
    # shellcheck disable=SC2016 # expanded by the deploy user's shell at login
    printf 'restrict,command="sudo -n /usr/local/sbin/wety-deploy $SSH_ORIGINAL_COMMAND" %s\n' \
        "$(head -n 1 "$deploy_key")" >/home/deploy/.ssh/authorized_keys
    chown deploy:deploy /home/deploy/.ssh/authorized_keys
    chmod 600 /home/deploy/.ssh/authorized_keys
fi

echo "== service"
systemctl daemon-reload
systemctl enable wety.service
# A restart reloads the graph (downtime): only when the unit changed.
if [ -L /srv/wety/current ] && [ "$unit_changed" = true ]; then systemctl restart wety.service; fi

echo "== done"
[ -L /srv/wety/current ] || echo "no release deployed yet: wety starts with the first deploy"
systemctl is-active --quiet cloudflared || echo "cloudflared is not running yet: install the tunnel token (deploy/README.md)"
