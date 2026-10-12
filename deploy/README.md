# Deployment

The API (`api.wety.org`) runs on an Oracle Cloud A1 instance (aarch64, Ubuntu 24.04).
The website (`wety.org`) is a Cloudflare Pages project. Pushing `main` deploys both,
server first.

## How it works

1. [`.github/workflows/deploy.yml`](../.github/workflows/deploy.yml) checks whether any
   server input (`SERVER_PATHS`) changed since the commit on the `production` branch.
   If so, it builds and tests the server on a GitHub ARM runner, uploads a release to
   R2 (`releases/<commit>/`), and runs `wety-deploy deploy <commit>` on the host over
   SSH.
2. [`wety-deploy`](wety-deploy) downloads the release and the graph it names,
   verifies their checksums, switches `/srv/wety/current`, restarts `wety.service`
   and waits until the server answers. If the server does not start, it switches back.
3. The workflow moves the `production` branch to the commit. Pages builds the website
   from `production` (API URL from `client/.env.production`), so the website only
   changes once its API is serving.

The graph is selected by [`graph.json`](graph.json), which records the graph's R2
object, checksum and artifact id. Change it with [`publish-graph.sh`](publish-graph.sh).

On the host, `cloudflared` runs the Cloudflare Tunnel `wety-api`: it connects out to
Cloudflare and forwards `api.wety.org` to the server on `127.0.0.1:3000`. TLS ends at
Cloudflare, so the host has no certificates. The host accepts only SSH, both in OCI's
network rules and in its own firewall. The server rate-limits by the
`CF-Connecting-IP` header the tunnel sets.

A release's `release.json` records the commit (and with it `Cargo.lock`), toolchain,
`RUSTFLAGS`, features and the graph's artifact id. The artifact's manifest leads to its
run receipt.

## Operating

- **Deploy:** merge into `main`. If only the website changed, the server is left alone.
- **Redeploy the server anyway:**

  ```bash
  gh workflow run deploy.yml -f force_server=true
  ```

- **New graph:** build it (`uv run --frozen wety-data run etymology_graph`), run
  `deploy/publish-graph.sh` from the main checkout (default: the artifact in
  `data/products/etymology_graph/CURRENT.json`; about 600 MB to upload), commit
  `deploy/graph.json`, merge into `main`. The server checks that the graph's
  language/POS tables match its own; if they do not, the deploy rolls back and fails.
  If the graph comes with a new Wiktionary language export, record the codes it
  retired first (root README, "Links"). Shared links name words, so they survive a new
  graph; open tabs notice it (the API answers 409) and reload.
- **Roll back:** run the command below on the host. It swaps `current` and `previous`.
  `production` still names the newer commit, so fix or revert on `main` next.

  ```bash
  sudo wety-deploy rollback
  ```

- **Inspect:** `sudo wety-deploy status`, `journalctl -u wety -f`,
  `journalctl -u cloudflared`. The deploy key can run `status` from your machine:
  `ssh -i ~/.ssh/wety_deploy deploy@<ip> status`.
- **After a restart:** the server needs a few minutes to load the graph. Until then
  `api.wety.org` returns a Cloudflare 502; that is expected. `wety-deploy` waits up to
  15 minutes (`WETY_HEALTH_TIMEOUT`) before it gives up.
- **Change the host setup** (`bootstrap.sh`, `wety-deploy`, `wety.service`): CI does
  not install these. Copy `deploy/` to the host again and rerun `bootstrap.sh`. It
  restarts the server only if the unit file changed.
- **OS updates:** `unattended-upgrades` installs security updates but does not reboot.
  Now and then run `sudo apt-get update && sudo apt-get full-upgrade` and reboot. A
  release upgrade (`sudo do-release-upgrade`) also works in place; the OCI console
  keeps showing the image the instance was created from.

## Setting up a host

For a new instance, or after replacing the boot volume (OCI → the instance → **More
actions** → **Replace boot volume**, Canonical Ubuntu 24.04 aarch64). Replacing the
boot volume keeps the instance, its shape and its public IP; a new A1 instance may hit
"out of capacity" errors. The R2 tokens, tunnel and GitHub environment already exist
(see [Reference](#accounts-and-secrets)). `<ip>` is the instance's public IP.

1. Log in, forgetting any old host key first, then update and reboot:

   ```bash
   ssh-keygen -R <ip>
   ```

   ```bash
   ssh ubuntu@<ip>
   ```

   ```bash
   sudo apt-get update && sudo apt-get full-upgrade -y && sudo reboot
   ```

   If SSH does not work, use the instance's **Console connection** in OCI.
2. Firewall: the host must accept only SSH. `netfilter-persistent` loads
   `/etc/iptables/rules.v4` and `rules.v6`. INPUT accepts established connections,
   ICMP, loopback and tcp/22, then rejects; FORWARD rejects; OUTPUT is open. OCI's
   Ubuntu images ship with similar rules; check with `sudo iptables -S` and
   `sudo ip6tables -S`, and make sure no other firewall (firewalld, ufw) is installed.
   `bootstrap.sh` does not set this up.
3. From the repository on your machine, copy `deploy/` and the deploy key's public
   half:

   ```bash
   scp -r deploy ~/.ssh/wety_deploy.pub ubuntu@<ip>:
   ```

4. On the host, run the bootstrap. It installs `cloudflared`, `rclone` and
   `unattended-upgrades`, creates the `wety` and `deploy` users, `/srv/wety`, the
   service, and the restricted deploy key:

   ```bash
   sudo bash deploy/bootstrap.sh ~/wety_deploy.pub
   ```

5. Fill in the `wety-deploy-host` R2 token (key id, secret, account id in the
   endpoint), then check that the host can read the bucket:

   ```bash
   sudo nano /etc/wety/r2.env
   ```

   ```bash
   sudo bash -c 'set -a; . /etc/wety/r2.env; rclone lsf -R r2:wety-deploy/graphs/'
   ```

6. Install the tunnel connector. The token is in Cloudflare → **Networking** →
   **Tunnels** → `wety-api` → install a connector (the long string after
   `cloudflared service install`). The tunnel should turn **Healthy**.

   ```bash
   sudo cloudflared service install <TUNNEL_TOKEN>
   ```

7. On your machine, record the new host key for GitHub. Compare the fingerprint that
   `ssh-keyscan` gets with the one the host reports, then store it:

   ```bash
   ssh-keyscan -t ed25519 <ip> | tee /tmp/wety_known_hosts | ssh-keygen -lf -
   ```

   ```bash
   ssh ubuntu@<ip> ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub
   ```

   ```bash
   gh secret set DEPLOY_KNOWN_HOSTS --env production < /tmp/wety_known_hosts
   ```

   If the IP changed, also `gh variable set DEPLOY_HOST --env production --body <ip>`.
8. Check that the deploy key can run `wety-deploy` and nothing else. The first command
   prints `current: none`; the second prints the usage message.

   ```bash
   ssh -i ~/.ssh/wety_deploy deploy@<ip> status
   ```

   ```bash
   ssh -i ~/.ssh/wety_deploy deploy@<ip> bash
   ```

9. Deploy. This builds `main`'s server if needed, deploys it, downloads the graph and
   loads it (several minutes), then moves `production`:

   ```bash
   gh workflow run deploy.yml -f force_server=true
   ```

   On the host, `curl -s 127.0.0.1:3000/periods | head -c 300` then answers, and so
   does `https://api.wety.org/resolve?lang=en&term=lime&ety=3`.

## Reference

Host layout (`/srv/wety`):

- `releases/<commit>/` holds `server`, `language_periods.json`, `release.json` and `env`.
  `env` sets `WETY_GRAPH_PATH` and `WETY_PERIODS_PATH`.
- `graphs/<sha256>/` holds a graph file.
- `current` and `previous` are symlinks to releases. Deploys keep these two plus the
  three newest releases, and the graphs those use.
- `/etc/wety/r2.env` holds the host's R2 token; `/etc/sudoers.d/wety-deploy` lets the
  `deploy` user run `wety-deploy` and nothing else.

R2 layout (bucket `wety-deploy`): `releases/<commit>/{release.json,bundle.tar.gz}` and
`graphs/<sha256>/<file>`. Your machine reaches it through the rclone remote
`wety-deploy`; on the host and in CI the remote is defined by `RCLONE_CONFIG_R2_*`
environment variables (`/etc/wety/r2.env`, the workflow) and is named `r2` there.

### Accounts and secrets

- **R2** (bucket `wety-deploy`, endpoint `https://<ACCOUNT_ID>.r2.cloudflarestorage.com`),
  three Account API tokens scoped to that bucket: `wety-deploy-ci` (Object Read &
  Write, in GitHub), `wety-deploy-host` (Object Read, in `/etc/wety/r2.env`) and
  `wety-deploy-local` (Object Read & Write, your machine's rclone remote). To recreate
  the remote (`rclone config` asks instead, keeping the secret out of shell history):

  ```bash
  rclone config create wety-deploy s3 provider=Cloudflare no_check_bucket=true access_key_id=<KEY_ID> secret_access_key=<SECRET> endpoint=https://<ACCOUNT_ID>.r2.cloudflarestorage.com
  ```

  The research store's bucket, `wety-research`, is separate
  ([provenance](../docs/architecture/provenance.md#setting-up-the-store)).
- **Cloudflare Tunnel** `wety-api`: its route publishes `api.wety.org` to
  `http://127.0.0.1:3000`, which also manages the `api` DNS record.
- **Cloudflare Pages:** the production branch is `production`. Preview builds, if
  enabled, talk to the production API.
- **GitHub** environment `production`, limited to `main`: secrets `R2_ACCESS_KEY_ID`,
  `R2_SECRET_ACCESS_KEY` (the `wety-deploy-ci` token), `DEPLOY_SSH_KEY` (the private
  half of `~/.ssh/wety_deploy`, an ed25519 key made for this) and `DEPLOY_KNOWN_HOSTS`;
  variables `R2_ENDPOINT`, `R2_BUCKET` (`wety-deploy`) and `DEPLOY_HOST` (the IP).
- **OCI:** the subnet's security list allows ingress on tcp/22 only. Oracle may reclaim
  idle instances on an Always Free account; Pay As You Go prevents that, and Always
  Free resources stay free.

Server environment:

| Variable | Meaning |
| --- | --- |
| `WETY_GRAPH_PATH` | graph file to serve |
| `WETY_PERIODS_PATH` | language periods file (default `data/products/dating/language_periods.json`) |
| `WETY_HOST`, `WETY_PORT` | listen address, default `0.0.0.0:3000`; `wety.service` sets `127.0.0.1:3000` |
| `WETY_CLIENT_IP_HEADER` | rate-limit by this header's IP, falling back to the peer address. Set it only when a proxy overwrites the header and is the only way in. |
| `WETY_ENVIRONMENT=production` | in-process TLS with `WETY_CERT_PATH`/`WETY_KEY_PATH`. Unused behind the tunnel. |

Release builds use `RUSTFLAGS="-C target-cpu=neoverse-n1"`. The host's Ampere Altra is
a Neoverse N1, while `.cargo/config.toml`'s `target-cpu=native` would target the build
machine. The current graph (600 MB gzipped JSON) takes about two minutes to load and
index on an M1 Pro and about 5 GB of memory once loaded; the host is slower.
