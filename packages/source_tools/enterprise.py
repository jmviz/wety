"""Wikimedia Enterprise snapshots: one project namespace's pages as NDJSON in a tar.gz,
each line the HTML and wikitext of one revision.

    uv run --frozen python -m source_tools.enterprise login USERNAME PASSWORD_FILE
    uv run --frozen python -m source_tools.enterprise info enwiktionary_namespace_0
    uv run --frozen python -m source_tools.enterprise snapshot enwiktionary_namespace_0

`login` runs once per 90 days; it keeps the tokens, never the password, in
~/.config/wety/enterprise.env (mode 600; WETY_ENTERPRISE_ENV names another file), and
every other command refreshes the 24-hour access token from it. A free account gets the
current month's snapshot only, 30 downloads a month, so a past snapshot exists only as
a retained artifact. `snapshot` acquires it into the collection `wiktionary-enterprise`
with the Snapshot Info it was taken under; tokens are never recorded.
"""

import argparse
import json
import os
import time
import urllib.request
from pathlib import Path

from source_tools.acquire import USER_AGENT, acquire
from workflow_tools.paths import REPO

AUTH = "https://auth.enterprise.wikimedia.com/v1"
API = "https://api.enterprise.wikimedia.com/v2"
COLLECTION = "wiktionary-enterprise"
# Refresh tokens last 90 days and give at most 90 access tokens.
REFRESH_DAYS = 90


def env_path():
    return Path(os.environ.get("WETY_ENTERPRISE_ENV") or "~/.config/wety/enterprise.env").expanduser()


def read_env(path):
    out = {}
    for line in path.read_text().splitlines():
        key, sep, value = line.partition("=")
        if sep and key.strip():
            out[key.strip()] = value.strip()
    return out


def write_env(path, values):
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as f:
        f.writelines(f"{k}={v}\n" for k, v in values.items())
    os.replace(tmp, path)


def post(url, body, token=None):
    request = urllib.request.Request(
        url,
        data=json.dumps(body).encode() if body is not None else b"",
        headers={"Content-Type": "application/json", "User-Agent": USER_AGENT},
        method="POST",
    )
    if token:
        request.add_unredirected_header("Authorization", "Bearer " + token)
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def login(username, password_file):
    password = Path(password_file).read_text().rstrip("\n")
    username = username.strip().lower()
    tokens = post(AUTH + "/login", {"username": username, "password": password})
    now = int(time.time())
    write_env(
        env_path(),
        {
            "WME_USERNAME": username,
            "WME_ACCESS_TOKEN": tokens["access_token"],
            "WME_ACCESS_EXPIRES": now + int(tokens.get("expires_in", 86400)),
            "WME_REFRESH_TOKEN": tokens["refresh_token"],
            "WME_REFRESH_EXPIRES": now + REFRESH_DAYS * 86400,
        },
    )


def access_token():
    """A valid access token, refreshed when it has under an hour left."""
    path = env_path()
    if not path.is_file():
        raise SystemExit(f"no Enterprise tokens in {path}: run `python -m source_tools.enterprise login`")
    env = read_env(path)
    now = int(time.time())
    if int(env.get("WME_ACCESS_EXPIRES", 0)) - now > 3600:
        return env["WME_ACCESS_TOKEN"]
    if int(env.get("WME_REFRESH_EXPIRES", 0)) <= now:
        raise SystemExit("the Enterprise refresh token has expired: run `python -m source_tools.enterprise login`")
    tokens = post(AUTH + "/token-refresh", {"username": env["WME_USERNAME"], "refresh_token": env["WME_REFRESH_TOKEN"]})
    env["WME_ACCESS_TOKEN"] = tokens["access_token"]
    env["WME_ACCESS_EXPIRES"] = str(now + int(tokens.get("expires_in", 86400)))
    write_env(path, env)
    return env["WME_ACCESS_TOKEN"]


def info(identifier):
    return post(f"{API}/snapshots/{identifier}", None, access_token())


def snapshot(identifier):
    """Acquire the snapshot as it stands now, with the Snapshot Info that names it."""
    meta = info(identifier)
    chunks = meta.pop("chunks", [])
    return acquire(
        f"{API}/snapshots/{identifier}/download",
        COLLECTION,
        version=f"{identifier} {meta['date_modified']} (version {meta['version']})",
        license="CC-BY-SA-4.0",
        authorization=lambda: "Bearer " + access_token(),
        attempts=20,
        provenance={"snapshot_info": {**meta, "chunk_count": len(chunks)}},
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("login")
    p.add_argument("username")
    p.add_argument("password_file", help="a file holding the password; it is read, never copied")
    sub.add_parser("info").add_argument("identifier")
    sub.add_parser("snapshot").add_argument("identifier")
    args = parser.parse_args()
    if args.command == "login":
        login(args.username, args.password_file)
        print("tokens in", env_path())
    elif args.command == "info":
        meta = info(args.identifier)
        meta["chunks"] = len(meta.get("chunks", []))
        print(json.dumps(meta, indent=1))
    else:
        print(snapshot(args.identifier).relative_to(REPO))


if __name__ == "__main__":
    main()
