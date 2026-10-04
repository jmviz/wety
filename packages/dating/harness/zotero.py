"""The maintainer's Zotero library, through Zotero desktop's local API.

Reads need Zotero's "Allow other applications on this computer to communicate with
Zotero" (Settings > Advanced). Writes (Zotero 10+) need a key from Zotero's own
authorization dialog; choose "Always Allow" and the key is kept in the system keyring,
never in the repository. The key covers the whole library, so this client confines
itself: it creates collections only under `wety`, creates items, and adds collections,
`wety:` tags and child notes to items. It never edits other fields or deletes anything.
Every write is read back, because a reported success is not proof of a stored change.

Zotero is the maintainer's interface, not evidence: files are copied into the holdings
store (holdings.py) and claims cite that retained text.
"""

import json
import os
from pathlib import Path
import urllib.error
import urllib.parse
import urllib.request

BASE = os.environ.get("WETY_ZOTERO_URL", "http://localhost:23119/api")
LIBRARY = "/users/0"  # the local user library
ROOT_COLLECTION = "wety"
TAG_PREFIX = "wety:"
KEYRING = ("wety-zotero-local-api", "key")
APP_NAME = "wety dating research"


class ZoteroError(RuntimeError):
    pass


class Zotero:
    def __init__(self, base=BASE, key=None):
        self.base = base.rstrip("/")
        self._key = key
        self._server_id = None
        # Never route localhost through a configured HTTP proxy.
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def request(self, method, path, params=None, body=None, headers=None, write=False):
        url = self.base + path + ("?" + urllib.parse.urlencode(params) if params else "")
        headers = dict(headers or {})
        data = None
        if body is not None:
            data = json.dumps(body).encode()
            headers["Content-Type"] = "application/json"
        if write:
            headers["Zotero-API-Key"] = self.key()
            headers["Zotero-Server-ID"] = self.server_id()
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with self.opener.open(req, timeout=60) as response:
                raw = response.read()
                return response.status, dict(response.headers), raw
        except urllib.error.HTTPError as exc:
            raw = exc.read()
            if exc.code == 403 and b"Local API is not enabled" in raw:
                raise ZoteroError(
                    "Zotero local API is off: Settings > Advanced > Allow other applications "
                    "on this computer to communicate with Zotero"
                ) from None
            return exc.code, dict(exc.headers), raw
        except urllib.error.URLError as exc:
            raise ZoteroError("Zotero is not running or unreachable at " + self.base) from exc

    def get(self, path, **params):
        params.setdefault("format", "json")
        status, _, raw = self.request("GET", LIBRARY + path, params)
        if status != 200:
            raise ZoteroError(f"GET {path}: HTTP {status}")
        return json.loads(raw)

    def get_all(self, path, **params):
        """Every page of a list endpoint."""
        items, start = [], 0
        while True:
            page = self.get(path, limit=100, start=start, **params)
            items += page
            if len(page) < 100:
                return items
            start += 100

    def server_id(self):
        if not self._server_id:
            _, headers, _ = self.request("GET", "/")
            self._server_id = {k.lower(): v for k, v in headers.items()}.get("zotero-server-id")
            if not self._server_id:
                raise ZoteroError("Zotero did not report a server ID; Zotero 10+ is required")
        return self._server_id

    # Reads

    def item(self, key):
        return self.get(f"/items/{key}")

    def children(self, key):
        return self.get_all(f"/items/{key}/children")

    def collections(self):
        return self.get_all("/collections")

    def collection_items(self, key):
        return self.get_all(f"/collections/{key}/items/top")

    def search(self, query):
        return self.get_all("/items/top", q=query, qmode="everything")

    def file_path(self, attachment_key):
        """Local path of an attachment's file, or None when it is not on this computer
        (e.g. synced metadata whose file was never downloaded)."""
        status, _, raw = self.request("GET", f"{LIBRARY}/items/{attachment_key}/file/view/url")
        if status == 404:
            return None
        if status != 200:
            raise ZoteroError(f"attachment {attachment_key}: HTTP {status}")
        url = raw.decode().strip()
        if not url.startswith("file://"):
            raise ZoteroError(f"attachment {attachment_key}: not a local file")
        path = Path(urllib.parse.unquote(urllib.parse.urlparse(url).path))
        return path if path.is_file() else None

    # Writes

    def key(self):
        if self._key:
            return self._key
        import keyring

        self._key = keyring.get_password(*KEYRING)
        if not self._key:
            self._key = self.authorize()
        return self._key

    def authorize(self):
        """Zotero shows Allow / Always Allow / Deny. Only "Always Allow" is reusable."""
        status, _, raw = self.request(
            "POST",
            "/local/authorize",
            body={"appName": APP_NAME},
            headers={"Zotero-Server-ID": self.server_id()},
        )
        if status != 200:
            raise ZoteroError(f"Zotero write authorization refused (HTTP {status})")
        answer = json.loads(raw)
        if answer.get("remember"):
            import keyring

            keyring.set_password(*KEYRING, answer["key"])
        return answer["key"]

    def create(self, kind, objects):
        """POST new items or collections; returns their keys, in order."""
        status, _, raw = self.request("POST", f"{LIBRARY}/{kind}", body=objects, write=True)
        if status != 200:
            raise ZoteroError(f"create {kind}: HTTP {status} {raw[:200]!r}")
        result = json.loads(raw)
        if result.get("failed"):
            raise ZoteroError(f"create {kind} failed: {json.dumps(result['failed'])[:300]}")
        keys = [result["successful"][str(i)]["key"] for i in range(len(objects))]
        for key in keys:  # read back
            self.get(f"/{kind}/{key}")
        return keys

    def add_to_item(self, key, collections=(), tags=()):
        """Add collections and wety tags to an item, leaving every other field as is."""
        if any(not t.startswith(TAG_PREFIX) for t in tags):
            raise ValueError("only wety: tags are written")
        item = self.item(key)
        data = item["data"]
        new_collections = sorted(set(data.get("collections", [])) | set(collections))
        old_tags = {t["tag"] for t in data.get("tags", [])}
        if set(new_collections) == set(data.get("collections", [])) and set(tags) <= old_tags:
            return item
        body = {
            "collections": new_collections,
            "tags": data.get("tags", []) + [{"tag": t} for t in tags if t not in old_tags],
        }
        status, _, raw = self.request(
            "PATCH",
            f"{LIBRARY}/items/{key}",
            body=body,
            headers={"If-Unmodified-Since-Version": str(item["version"])},
            write=True,
        )
        if status not in (200, 204):
            raise ZoteroError(f"update {key}: HTTP {status} {raw[:200]!r}")
        after = self.item(key)["data"]
        if not set(new_collections) <= set(after.get("collections", [])) or not set(tags) <= {
            t["tag"] for t in after.get("tags", [])
        }:
            raise ZoteroError(f"update {key} reported success but was not stored")
        return after

    def remove_tag(self, key, tag):
        if not tag.startswith(TAG_PREFIX):
            raise ValueError("only wety: tags are removed")
        item = self.item(key)
        tags = [t for t in item["data"].get("tags", []) if t["tag"] != tag]
        if len(tags) == len(item["data"].get("tags", [])):
            return
        status, _, raw = self.request(
            "PATCH",
            f"{LIBRARY}/items/{key}",
            body={"tags": tags},
            headers={"If-Unmodified-Since-Version": str(item["version"])},
            write=True,
        )
        if status not in (200, 204):
            raise ZoteroError(f"update {key}: HTTP {status} {raw[:200]!r}")

    def ensure_collection(self, path):
        """Key of the collection at path (a list of names under `wety`), creating it."""
        if not path or path[0] != ROOT_COLLECTION:
            raise ValueError("collections are written only under " + ROOT_COLLECTION)
        existing = self.collections()
        parent = False
        for name in path:
            match = next(
                (
                    c["key"]
                    for c in existing
                    if c["data"]["name"] == name and c["data"].get("parentCollection") == parent
                ),
                None,
            )
            if not match:
                match = self.create("collections", [{"name": name, "parentCollection": parent}])[0]
                existing.append({"key": match, "data": {"name": name, "parentCollection": parent}})
            parent = match
        return parent

    def collection_tree(self, root=ROOT_COLLECTION):
        """Keys of `root` and every collection below it."""
        existing = self.collections()
        keys = {
            c["key"]
            for c in existing
            if c["data"]["name"] == root and c["data"].get("parentCollection") is False
        }
        grew = True
        while grew:
            more = {c["key"] for c in existing if c["data"].get("parentCollection") in keys}
            grew = not more <= keys
            keys |= more
        return keys
