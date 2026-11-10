"""Wiktionary page wikitext: pages from a pages-articles dump, and templates.

Only what the bibliography needs: `Template:R:` pages, and templates with their
arguments split at the top level.
"""

import html
import re

COMMENT = re.compile(r"<!--.*?(?:-->|$)", re.DOTALL)
REF_TAG = re.compile(r"<ref\b[^>/]*/>|<ref\b[^>]*>.*?</ref\s*>", re.DOTALL | re.IGNORECASE)
REDIRECT = re.compile(r"^\s*#redirect\s*:?\s*\[\[([^\]|#]+)", re.IGNORECASE)

# Namespaces read: entries, reconstructions and templates.
MAIN, TEMPLATE, RECONSTRUCTION = "0", "10", "118"


def pages(lines, namespaces=(MAIN, TEMPLATE, RECONSTRUCTION)):
    """(title, namespace, text) for each page of a pages-articles XML dump, read by line."""
    title = ns = None
    buf = None
    for line in lines:
        if buf is not None:
            end = line.find("</text>")
            if end < 0:
                buf.append(line)
                continue
            buf.append(line[:end])
            if ns in namespaces:
                yield title, ns, html.unescape("".join(buf))
            buf = None
            continue
        s = line.lstrip()
        if s.startswith("<title>"):
            title = html.unescape(s[7 : s.index("</title>")])
            ns = None
        elif s.startswith("<ns>"):
            ns = s[4 : s.index("</ns>")]
        elif s.startswith("<text") and ns in namespaces:
            if s.rstrip().endswith("/>"):
                yield title, ns, ""
                continue
            body = s[s.index(">") + 1 :]
            end = body.find("</text>")
            if end >= 0:
                yield title, ns, html.unescape(body[:end])
            else:
                buf = [body]


def redirect(text):
    m = REDIRECT.match(text)
    return m.group(1).strip() if m else None


def templates(text, prefix=""):
    """(name, args) of each outermost template whose name starts with `prefix`.

    `args` is the list of top-level arguments after the name, unstripped; nested
    templates, links and parser functions inside arguments are kept as written.
    """
    out = []
    i = 0
    while True:
        i = text.find("{{", i)
        if i < 0:
            return out
        end = _close(text, i)
        if end is None:
            return out
        inner = text[i + 2 : end - 2]
        parts = _split(inner)
        name = normalize_name(parts[0])
        if name.startswith(prefix) and not inner.startswith("{"):
            out.append((name, parts[1:]))
        i = end


def normalize_name(name):
    name = re.sub(r"[\s_]+", " ", name).strip()
    if name.lower().startswith("template:"):
        name = name[9:].lstrip()
    return name


def named_args(args):
    """Positional args numbered from 1, and named ones, as one dict of stripped values."""
    out, n = {}, 0
    for a in args:
        k, eq, v = a.partition("=")
        if eq and re.fullmatch(r"\s*[^{}\[\]|<>]+?\s*", k):
            out[k.strip()] = v.strip()
        else:
            n += 1
            out[str(n)] = a.strip()
    return out


def strip_refs(text):
    """The text with its <ref> footnotes removed, and how many there were."""
    return REF_TAG.subn("", text)


def _tokens(text, i):
    """(index, kind, depth after) for each brace or link token from i: kind is "open" or
    "close", for templates and links (two braces or brackets) and parameters (three
    braces). A closer of three braces closes a parameter only when one is open."""
    stack = []
    j, n = i, len(text)
    while j < n:
        c = text[j]
        if c == "{" and text.startswith("{{", j):
            if text.startswith("{{{", j) and not text.startswith("{{{{", j + 1):
                stack.append(3)
                j += 3
            else:
                stack.append(2)
                j += 2
            yield j, "open", len(stack)
        elif c == "[" and text.startswith("[[", j):
            stack.append("[")
            j += 2
            yield j, "open", len(stack)
        elif c == "}" and text.startswith("}}", j) and stack and stack[-1] != "[":
            width = stack.pop()
            j += width if text.startswith("}" * width, j) else 2
            yield j, "close", len(stack)
        elif c == "]" and text.startswith("]]", j) and stack and stack[-1] == "[":
            stack.pop()
            j += 2
            yield j, "close", len(stack)
        else:
            j += 1
            if c == "|" and len(stack) == 1:
                yield j, "pipe", 1


def _close(text, i):
    """The index just after the template opened by the braces at i."""
    for j, kind, depth in _tokens(text, i):
        if kind == "close" and depth == 0:
            return j
    return None


def _split(inner):
    """Split a template's inside at the pipes not nested in braces or links."""
    wrapped = "{{" + inner + "}}"
    parts, start = [], 2
    for j, kind, depth in _tokens(wrapped, 0):
        if kind == "pipe":
            parts.append(wrapped[start : j - 1])
            start = j
    parts.append(wrapped[start:-2])
    return parts
