"""Assertion files: building their objects and validating them.

An assertions file is JSON Lines; each line is one object of `schema/assertion.schema.json`,
told apart by `type`. The schema checks each line; `validate` also checks what spans
lines: every id is unique, every reference resolves to an object of the right type, and
every digest id equals the digest of the fields it is made from.

Ids that a digest makes:

- a hypothesis: its word and links, so two sources that claim the same thing share it;
- an alternative set: its members, exclusivity and preference;
- an assertion: its source, locator, target and stance;
- an extraction: its method, tool, revision, dirtiness, inputs, model and reviewer.

An assertion that a wety rule compiled from a source's structure says so in `compiled`;
only a rule may compile, and only hypotheses.
"""

import argparse
import collections
import hashlib
import json
import subprocess
import sys
from functools import cache
from pathlib import Path

import jsonschema

SCHEMA_PATH = Path(__file__).parent / "schema" / "assertion.schema.json"
EXTENSIONS_PATH = Path(__file__).parent / "schema" / "extensions.json"
REPO = Path(__file__).resolve().parents[2]
ORDER = (
    "source",
    "agent",
    "work",
    "extraction",
    "record",
    "hypothesis",
    "cognate_set",
    "alternative_set",
    "assertion",
)


def digest(value):
    text = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(text.encode()).hexdigest()[:16]


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def hypothesis(word, links):
    return {
        "type": "hypothesis",
        "id": "hyp:" + digest([word, links]),
        "word": word,
        "links": links,
    }


def alternative_set(members, exclusive, preference=None):
    key = [members, exclusive, preference]
    return {
        "type": "alternative_set",
        "id": "alt:" + digest(key),
        "members": members,
        "exclusive": exclusive,
        "preference": preference,
    }


def assertion(source, locator, target, stance, **fields):
    a = {
        "type": "assertion",
        "id": "as:" + digest([source, locator, target, stance]),
        "source": source,
        "stance": stance,
        "target": target,
        "locator": locator,
    }
    a.update({k: v for k, v in fields.items() if v not in (None, [], {})})
    return a


def extraction(method, tool, inputs, model=None, reviewer=None):
    """The current checkout's extraction record for `tool` over `inputs` (paths)."""
    revision = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.strip()
    dirty = bool(
        subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=no"],
            cwd=REPO,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    )
    e = {
        "type": "extraction",
        "method": method,
        "tool": tool,
        "revision": revision,
        "dirty": dirty,
        "inputs": [
            {"path": str(Path(p).resolve().relative_to(REPO)), "sha256": sha256_file(p)}
            for p in inputs
        ],
    }
    if method == "model":
        e["model"] = model
        e["reviewer"] = reviewer
    return {"id": "extraction:" + digest({k: v for k, v in e.items() if k != "type"}), **e}


class Writer:
    """Collects one file's objects; an id given twice must carry the same object."""

    def __init__(self):
        self.objects = {}

    def add(self, obj):
        old = self.objects.setdefault(obj["id"], obj)
        if old != obj:
            raise ValueError(f"{obj['id']} given twice with different content")
        return obj["id"]

    def write(self, path):
        rank = {t: i for i, t in enumerate(ORDER)}
        objs = sorted(self.objects.values(), key=lambda o: rank[o["type"]])
        with open(path, "w", encoding="utf-8") as f:
            f.writelines(json.dumps(o, ensure_ascii=False, sort_keys=True) + "\n" for o in objs)
        return len(objs)


@cache
def validator():
    schema = json.loads(SCHEMA_PATH.read_text())
    jsonschema.Draft202012Validator.check_schema(schema)
    return jsonschema.Draft202012Validator(schema)


@cache
def extensions():
    """The declared extensions: (source, object type, field) to their registry entry."""
    rows = json.loads(EXTENSIONS_PATH.read_text())["extensions"]
    return {(r["source"], r["object"], r["field"]): r for r in rows}


BRANCHES = {
    "source": "Source",
    "agent": "Agent",
    "work": "Work",
    "extraction": "Extraction",
    "record": "Record",
    "hypothesis": "Hypothesis",
    "cognate_set": "CognateSet",
    "alternative_set": "AlternativeSet",
    "assertion": "Assertion",
}


@cache
def branch_validator(branch):
    schema = validator().schema
    return jsonschema.Draft202012Validator({"$ref": f"#/$defs/{branch}", "$defs": schema["$defs"]})


def schema_errors(obj):
    """The schema's complaints about one object, from the branch its `type` selects."""
    branch = BRANCHES.get(obj.get("type"))
    if branch is None:
        return [f"unknown type {obj.get('type')!r}"]
    return [
        f"{'/'.join(map(str, e.absolute_path)) or '(top)'}: {e.message}"
        for e in branch_validator(branch).iter_errors(obj)
    ]


def cross_errors(objs):
    """What the schema cannot see: uniqueness, references, digests and declared extensions."""
    errors = []
    by_id = {}
    for o in objs:
        if o["id"] in by_id:
            errors.append(f"{o['id']}: id used twice")
        by_id[o["id"]] = o

    def need(ref, types, where):
        o = by_id.get(ref)
        if o is None or o["type"] not in types:
            errors.append(f"{where}: {ref} is not a {' or '.join(types)} in this file")
            return None
        return o

    sources = {o["id"].split("@")[0] for o in objs if o["type"] == "source"}
    for o in objs:
        i, t = o["id"], o["type"]
        for name, ext in o.get("x", {}).items():
            if name not in sources:
                errors.append(f"{i}: extension {name!r} names no source in this file")
            for field in ext:
                if (name, t, field) not in extensions():
                    errors.append(
                        f"{i}: x.{name}.{field} is not declared in schema/extensions.json"
                    )
        if t in ("record", "cognate_set", "assertion"):
            need(o["source"], ("source",), i)
        if t in ("record", "cognate_set") and i.split(":")[0] != o["source"].split("@")[0]:
            errors.append(f"{i}: id does not start with its source's name")
        if t == "work":
            for a in o.get("authors", []):
                need(a, ("agent",), i)
        elif t == "extraction":
            body = {k: v for k, v in o.items() if k not in ("type", "id")}
            if i != "extraction:" + digest(body):
                errors.append(f"{i}: id is not the digest of its fields")
        elif t == "hypothesis":
            if i != "hyp:" + digest([o["word"], o["links"]]):
                errors.append(f"{i}: id is not the digest of its word and links")
            if o["links"][0]["child"] != o["word"]:
                errors.append(f"{i}: the first link's child is not the word")
            reached = {o["word"]}
            for n, link in enumerate(o["links"]):
                if link["child"] not in reached:
                    errors.append(f"{i}: link {n}'s child is not the word or an earlier parent")
                if link.get("head", 0) >= max(len(link["parents"]), 1):
                    errors.append(f"{i}: link {n}'s head is not one of its parents")
                for r in (link["child"], *link["parents"]):
                    need(r, ("record",), f"{i} link {n}")
                reached.update(link["parents"])
        elif t == "cognate_set":
            if o["reconstruction"] is not None:
                need(o["reconstruction"], ("record",), i)
            elif not o["members"]:
                errors.append(f"{i}: neither a reconstruction nor members")
            for r in o["members"]:
                need(r, ("record",), i)
        elif t == "alternative_set":
            if i != "alt:" + digest([o["members"], o["exclusive"], o["preference"]]):
                errors.append(f"{i}: id is not the digest of its members")
            for m in o["members"]:
                need(m, ("hypothesis", "cognate_set"), i)
            if o["preference"] is not None and not set(o["preference"]) <= set(o["members"]):
                errors.append(f"{i}: preference names a non-member")
        elif t == "assertion":
            if i != "as:" + digest([o["source"], o["locator"], o["target"], o["stance"]]):
                errors.append(f"{i}: id is not the digest of source, locator, target, stance")
            agents = {role: need(o[role], ("agent",), i) for role in ("asserter", "holder")}
            need(o["extraction"], ("extraction",), i)
            target = need(o["target"], ("hypothesis", "cognate_set", "alternative_set"), i)
            if "compiled" in o:
                if agents["asserter"] and agents["asserter"]["kind"] != "rule":
                    errors.append(f"{i}: compiled, but its asserter is not a rule")
                if target and target["type"] != "hypothesis":
                    errors.append(f"{i}: compiled, but its target is not a hypothesis")
            for c in o.get("credits", []):
                need(c["work"], ("work",), i)
            for alt in o.get("in_sets", []):
                s = need(alt, ("alternative_set",), i)
                if s and o["target"] not in s["members"]:
                    errors.append(f"{i}: its target is not a member of {alt}")
            if "replaces" in o:
                need(o["replaces"], ("assertion",), i)
            for h in o.get("hedges", []):
                if target is None:
                    break
                if "link" in h and (
                    target["type"] != "hypothesis" or h["link"] >= len(target["links"])
                ):
                    errors.append(f"{i}: hedge on link {h['link']}, which its target lacks")
                if "member" in h and (
                    target["type"] != "cognate_set" or h["member"] >= len(target["members"])
                ):
                    errors.append(f"{i}: hedge on member {h['member']}, which its target lacks")
    return errors


def read(path):
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def validate(objs):
    """Every error in a file's objects: schema errors first, by line, then cross-record."""
    errors = []
    for n, o in enumerate(objs, 1):
        errors += [f"line {n} ({o.get('id')}): {e}" for e in schema_errors(o)]
    if not errors:
        errors += cross_errors(objs)
    return errors


def summary(objs):
    """Counts a check reports: objects by type, assertions by stance and target, hedges,
    credits, and the extension fields each source uses."""
    by_id = {o["id"]: o for o in objs}
    s = collections.defaultdict(collections.Counter)
    for o in objs:
        s["objects"][o["type"]] += 1
        for name, ext in o.get("x", {}).items():
            for field in ext:
                s["extensions"][f"{name}: {o['type']}.{field}"] += 1
        if o["type"] == "record" and "certainty" in o:
            s["records"][f"reconstruction {o['certainty']['level']}"] += 1
        if o["type"] == "record" and o["form"] is None:
            s["records"]["language only, no form"] += 1
        if o["type"] == "hypothesis":
            for link in o["links"]:
                s["links"][link["relation"] + (f" ({link['mode']})" if "mode" in link else "")] += 1
        if o["type"] == "cognate_set":
            s["cognate_sets"]["named" if o["reconstruction"] else "unnamed"] += 1
        if o["type"] == "alternative_set":
            s["alternative_sets"]["exclusive" if o["exclusive"] else "not exclusive"] += 1
        if o["type"] == "assertion":
            target = by_id.get(o["target"], {}).get("type")
            s["assertions"][f"{o['stance']} {target}"] += 1
            if o["holder"] != o["asserter"]:
                s["assertions"]["holder is not the asserter"] += 1
            if "in_sets" in o:
                s["assertions"]["in an alternative set"] += 1
            if "compiled" in o:
                s["assertions"]["compiled"] += 1
            for h in o.get("hedges", []):
                on = "link" if "link" in h else "member" if "member" in h else "unplaced"
                s["hedges"][f"{h['level']} on {on}"] += 1
            for c in o.get("credits", []):
                s["credits"][f"{c['role']}, {c['scope']}"] += 1
    return {k: dict(sorted(v.items())) for k, v in s.items()}


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("files", nargs="+", type=Path)
    p.add_argument("--summary", action="store_true", help="print counts as JSON")
    args = p.parse_args(argv)
    failed = False
    for path in args.files:
        objs = read(path)
        errors = validate(objs)
        print(f"{path}: {len(objs)} objects, {len(errors)} errors")
        for e in errors[:50]:
            print("  " + e)
        failed |= bool(errors)
        if args.summary:
            print(json.dumps(summary(objs), ensure_ascii=False, indent=1))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
