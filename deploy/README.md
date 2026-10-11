# Deployment

The API (`api.wety.org`) runs on an Oracle Cloud A1 instance (Ubuntu 24.04, aarch64).
The website (`wety.org`) is a Cloudflare Pages project. Pushing `main` deploys both,
server first:

1. [`.github/workflows/deploy.yml`](../.github/workflows/deploy.yml) checks whether any
   server input changed since the commit on the `production` branch. If so, it builds
   and tests the server on a GitHub ARM runner, uploads a release to R2
   (`releases/<commit>/`), and runs `wety-deploy deploy <commit>` on the host over SSH.
2. [`wety-deploy`](wety-deploy) downloads the release and the graph it names,
   verifies their checksums, switches `/srv/wety/current`, restarts `wety.service`
   and waits until the server answers. If the server does not start, it switches back.
3. The workflow moves the `production` branch to the commit. Pages builds the
   website from `production`, so the website only changes once its API is serving.

The graph is selected by [`graph.json`](graph.json), which records the graph's R2
object, checksum and artifact id. Change it with [`publish-graph.sh`](publish-graph.sh).

On the host, `cloudflared` connects out to Cloudflare and forwards `api.wety.org` to
`127.0.0.1:3000`. TLS ends at Cloudflare, so the host has no certificates and opens no
ports except SSH. The server rate-limits by the `CF-Connecting-IP` header the tunnel
sets.

A release's `release.json` records the commit (and with it `Cargo.lock`), toolchain,
`RUSTFLAGS`, features and the graph's artifact id. The artifact's manifest leads to its
run receipt.

## One-time setup

The first deployment moves the site to the current graph, server and website at once:
links now name words rather than numeric IDs, and the new website needs the new API
(`/resolve`, language codes). Steps 1–5 do not touch the running site; step 4 makes
sure of that by holding the website back. The site is down from step 6 until step 7
is done, probably 30–60 minutes. `<ip>` is the instance's public IP.

### 1. R2: bucket, tokens, graph upload

1. Cloudflare dashboard → **R2 Object Storage**. If R2 is not enabled yet, enable it.
   Cloudflare may ask for a payment method; usage stays within the free tier (10 GB).
2. **Create bucket**: name `wety-deploy`, location automatic.
3. On the R2 **Overview** page, copy the **Account ID**. The endpoint is
   `https://<ACCOUNT_ID>.r2.cloudflarestorage.com`.
4. **Overview** → **API Tokens** → **Manage** → **Create Account API token**, three
   times, each scoped to the `wety-deploy` bucket only. Each token's secret is shown once, so
   copy both values each time:
   - `wety-deploy-ci`: **Object Read & Write**. Goes to GitHub (step 3).
   - `wety-deploy-host`: **Object Read**. Goes to the host (step 6).
   - `wety-deploy-local`: **Object Read & Write**. Used for graph uploads from your machine.
5. On your machine, install rclone and add a remote named `wety-deploy` with the `wety-deploy-local`
   token. The command line puts the secret in shell history; run `rclone config`
   instead if you prefer to answer prompts.

   ```bash
   brew install rclone
   ```

   ```bash
   rclone config create wety-deploy s3 provider=Cloudflare no_check_bucket=true access_key_id=<KEY_ID> secret_access_key=<SECRET> endpoint=https://<ACCOUNT_ID>.r2.cloudflarestorage.com
   ```

6. Upload the selected graph (the artifact named in
   `data/products/etymology_graph/CURRENT.json`) from the main checkout, where `data/`
   lives. Expect a 600 MB upload.

   ```bash
   deploy/publish-graph.sh
   ```

   `git diff deploy/graph.json` should then show no change: the committed file already
   names this graph. If it does change, commit it.

### 2. Cloudflare Tunnel

The tunnel needs `wety.org`'s DNS on Cloudflare. Check **wety.org** → **DNS** →
**Records**: there should be an `api` A record pointing to `<ip>`.

1. **Networking** → **Tunnels** → **Create a tunnel**. Pick **Cloudflared** if asked,
   name it `wety-api`, and create it.
2. The next page shows install commands for an operating system. Do not run them.
   Copy the token: the long string after `cloudflared service install`. You need it in
   step 6.
3. Do not add a route yet. Adding it replaces the `api` DNS record, so it happens in
   step 7.

### 3. GitHub

1. Create the deploy key, which may only run `wety-deploy` on the host:

   ```bash
   ssh-keygen -t ed25519 -N '' -C wety-deploy -f ~/.ssh/wety_deploy
   ```

2. Repository **Settings** → **Environments** → **New environment** `production`. Under
   **Deployment branches and tags**, choose **Selected branches and tags** and add
   `main`.
3. Add the environment's secrets and variables. With `gh`, each `secret set` prompts for
   its value:

   ```bash
   gh secret set R2_ACCESS_KEY_ID --env production
   ```

   ```bash
   gh secret set R2_SECRET_ACCESS_KEY --env production
   ```

   ```bash
   gh secret set DEPLOY_SSH_KEY --env production < ~/.ssh/wety_deploy
   ```

   ```bash
   gh variable set R2_ENDPOINT --env production --body https://<ACCOUNT_ID>.r2.cloudflarestorage.com
   ```

   ```bash
   gh variable set R2_BUCKET --env production --body wety-deploy
   ```

   ```bash
   gh variable set DEPLOY_HOST --env production --body <ip>
   ```

   The R2 values are those of the `wety-deploy-ci` token. `DEPLOY_KNOWN_HOSTS` follows in step 6.

### 4. Hold the website back

Pages builds the website from `main` today. Point it at a `production` branch that
names what is live now, so that pushing `main` in step 5 does not put the new website
in front of the old API. From now on the workflow moves `production` once the server
it needs is up.

1. Create `production` at the commit that is live:

   ```bash
   git fetch origin
   ```

   ```bash
   git push origin origin/main:refs/heads/production
   ```

2. Pages project → **Settings** → **Builds & deployments** → **Configure Production
   deployments**: set the production branch to `production`. Pages rebuilds the same
   website.
3. Optionally set automatic **preview** deployments to **None**. Otherwise every push
   to `main` also builds a preview, which talks to the production API.

### 5. Build the first release

Push `main`. The workflow builds the server and uploads the release to R2. Its
**Deploy** step then fails, because the host is not set up yet; that is expected, and
nothing changes on the old host or the website. Note the run id:

```bash
git push origin main
```

```bash
gh run list --workflow deploy.yml --limit 1
```

### 6. Rebuild the host on Ubuntu 24.04 (site down from here)

The current image is Ubuntu 22.04, whose standard support ends in 2027. Replacing the
boot volume keeps the instance, its shape and its IP; creating a new A1 instance
instead risks "out of capacity" errors.

1. OCI Console → **Compute** → **Instances** → the instance → **More actions** →
   **Replace boot volume**. Pick **Image**, then Canonical Ubuntu 24.04 (the aarch64
   build), size 47 GB, and enable **Preserve boot volume** so the old disk survives as a
   detached boot volume. If the dialog asks for SSH keys, give your usual public key.
2. Once the instance is running again, forget its old host key and log in:

   ```bash
   ssh-keygen -R <ip>
   ```

   ```bash
   ssh ubuntu@<ip>
   ```

   If SSH does not work, use the instance's **Console connection** in OCI. If the
   replacement went wrong, the preserved volume can be attached back the same way.
3. On the host, update and reboot:

   ```bash
   sudo apt-get update && sudo apt-get full-upgrade -y && sudo reboot
   ```

4. From this checkout on your machine, copy the `deploy/` directory and the deploy key's
   public half:

   ```bash
   scp -r deploy ~/.ssh/wety_deploy.pub ubuntu@<ip>:
   ```

5. On the host, run the bootstrap. It installs packages, creates the `wety` and `deploy`
   users, `/srv/wety`, the service, and the restricted deploy key:

   ```bash
   sudo bash deploy/bootstrap.sh ~/wety_deploy.pub
   ```

6. Fill in the `wety-deploy-host` R2 token (key id, secret, account id in the endpoint):

   ```bash
   sudo nano /etc/wety/r2.env
   ```

   Check that the host can read the bucket:

   ```bash
   sudo bash -c 'set -a; . /etc/wety/r2.env; rclone lsf -R r2:wety-deploy/graphs/'
   ```

7. Install the tunnel connector with the token from step 2:

   ```bash
   sudo cloudflared service install <TUNNEL_TOKEN>
   ```

   The tunnel's status in **Networking** → **Tunnels** should turn **Healthy**.
8. On your machine, record the host key for GitHub. Compare the fingerprint that
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

9. Check that the deploy key can run `wety-deploy` and nothing else. The first command
   prints `current: none`; the second prints the usage message.

   ```bash
   ssh -i ~/.ssh/wety_deploy deploy@<ip> status
   ```

   ```bash
   ssh -i ~/.ssh/wety_deploy deploy@<ip> bash
   ```

10. Deploy the release from step 5 on the host. It downloads the 600 MB graph and
    loads it, which takes a few minutes. `<commit>` is the `main` commit that step 5
    built (`git rev-parse origin/main` on your machine).

    ```bash
    sudo wety-deploy deploy <commit>
    ```

    ```bash
    curl -s 127.0.0.1:3000/periods | head -c 300
    ```

11. On your machine, rerun the failed job. It finds the release already serving,
    then moves `production` to it, and Pages builds the new website:

    ```bash
    gh run rerun <run-id> --failed
    ```

### 7. Point api.wety.org at the tunnel (site back up)

1. **wety.org** → **DNS** → **Records**: delete the `api` A record.
2. **Networking** → **Tunnels** → `wety-api` → **Routes** → **Add route** →
   **Published application**: subdomain `api`, domain `wety.org`, service URL
   `http://127.0.0.1:3000`. Add the route.
3. Check, then load a word, e.g. https://wety.org/etymology/en/lime?ety=3 (a browser
   tab still holding the old website may need a reload):

   ```bash
   curl -s 'https://api.wety.org/resolve?lang=en&term=lime&ety=3' | head -c 300
   ```

### 8. Check the website follows `production`

With the next push to `main`, the workflow's `promote` job moves `production`, and
Pages starts a production build of that commit.

### 9. Clean up

- OCI → **Networking** → the VCN → the subnet's security list: delete the ingress rules
  for ports 80 and 443. Keep 22.
- Once the new host has run well for a while, delete the preserved 22.04 boot volume
  (**Block Storage** → **Boot Volumes**).
- If the OCI account is still the Always Free type rather than Pay As You Go, Oracle may
  reclaim instances it considers idle. Upgrading to Pay As You Go prevents that; Always
  Free resources stay free.

## Operating

- **Deploy:** merge into `main`. If only the website changed, the server is left alone.
- **Redeploy the server anyway:**

  ```bash
  gh workflow run deploy.yml -f force_server=true
  ```

- **New graph:** build it (`uv run --frozen wety-data run etymology_graph`), run
  `deploy/publish-graph.sh` (default: the artifact in `CURRENT.json`), commit
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
  `journalctl -u cloudflared`.
- **Change the host setup** (`bootstrap.sh`, `wety-deploy`, `wety.service`): CI does
  not install these. Copy `deploy/` to the host again and rerun `bootstrap.sh`. It
  restarts the server only if the unit file changed.
- **OS updates:** `unattended-upgrades` installs security updates but does not reboot.
  Now and then run `sudo apt-get full-upgrade` and reboot. Each restart reloads the
  graph, so the API is down while it loads.

## Reference

Host layout (`/srv/wety`):

- `releases/<commit>/` holds `server`, `language_periods.json`, `release.json` and `env`.
  `env` sets `WETY_GRAPH_PATH` and `WETY_PERIODS_PATH`.
- `graphs/<sha256>/` holds a graph file.
- `current` and `previous` are symlinks to releases. Deploys keep these two plus the
  three newest releases, and the graphs those use.

R2 layout (bucket `wety-deploy`): `releases/<commit>/{release.json,bundle.tar.gz}` and
`graphs/<sha256>/<file>`. Your machine reaches it through the rclone remote
`wety-deploy`; on the host and in CI the remote is defined by `RCLONE_CONFIG_R2_*`
environment variables (`/etc/wety/r2.env`, the workflow) and is named `r2` there.

Server environment:

| Variable | Meaning |
| --- | --- |
| `WETY_GRAPH_PATH` | graph file to serve |
| `WETY_PERIODS_PATH` | language periods file (default `data/products/dating/language_periods.json`) |
| `WETY_HOST`, `WETY_PORT` | listen address, default `0.0.0.0:3000` |
| `WETY_CLIENT_IP_HEADER` | rate-limit by this header's IP, falling back to the peer address. Set it only when a proxy overwrites the header and is the only way in. |
| `WETY_ENVIRONMENT=production` | in-process TLS with `WETY_CERT_PATH`/`WETY_KEY_PATH`. Unused behind the tunnel. |

Release builds use `RUSTFLAGS="-C target-cpu=neoverse-n1"`. The host's Ampere Altra is
a Neoverse N1, while `.cargo/config.toml`'s `target-cpu=native` would target the build
machine. The current graph (600 MB gzipped JSON) takes about two minutes to load and
index on an M1 Pro and about 5 GB of memory once loaded; the host is slower.
`wety-deploy` waits up to 15 minutes (`WETY_HEALTH_TIMEOUT`).
The old PyTorch-era instructions are in git history
(`git show c472620:server/deployment.md`).
