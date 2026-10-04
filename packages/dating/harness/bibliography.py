"""Full bibliographic records for requested works: identifiers first, then metadata.

Zotero desktop's "Add Item by Identifier" runs translators (doi.org/Crossref, library
catalogues such as K10plus); its local API offers no lookup, so this does the same:
- a DOI resolves through doi.org content negotiation (CSL JSON from Crossref, DataCite);
- an ISBN through K10plus (SRU, MODS), the union catalogue Zotero also queries;
- a work with neither is searched in Crossref, then K10plus, by title, author and year,
  and a candidate is accepted only with the same title (subtitle optional), the same
  first author or editor surname, and the same year: an edition is not interchangeable.
Place, edition, pages and series missing from Crossref are filled from K10plus by ISBN.
The result is Zotero item data and the provenance of each part.
"""

import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

USER_AGENT = "wety-sources/1.0 (https://github.com/jmviz/wety)"
K10PLUS = "https://sru.k10plus.de/opac-de-627"
MODS = {"m": "http://www.loc.gov/mods/v3"}
PACE = 1.0  # seconds between requests to one host
_last = {}

CSL_TYPES = {
    "book": "book",
    "monograph": "book",
    "edited-book": "book",
    "reference-book": "book",
    "book-chapter": "bookSection",
    "chapter": "bookSection",
    "book-section": "bookSection",
    "book-part": "bookSection",
    "reference-entry": "bookSection",
    "journal-article": "journalArticle",
    "article-journal": "journalArticle",
    "dissertation": "thesis",
    "thesis": "thesis",
    "report": "report",
    "dataset": "dataset",
}
WANTED = {
    "book": {"book", "monograph", "edited-book", "reference-book"},
    "bookSection": {"book-chapter", "book-section", "book-part", "reference-entry"},
    "journalArticle": {"journal-article"},
}
ROLES = {"aut": "author", "edt": "editor", "trl": "translator", "ctb": "contributor"}


def get(url, accept=None, raw=False):
    host = urllib.parse.urlparse(url).hostname
    for attempt in (1, 2, 3):
        wait = _last.get(host, 0) + PACE * attempt - time.time()
        if wait > 0:
            time.sleep(wait)
        _last[host] = time.time()
        headers = {"User-Agent": USER_AGENT, **({"Accept": accept} if accept else {})}
        try:
            with urllib.request.urlopen(
                urllib.request.Request(url, headers=headers), timeout=60
            ) as r:
                body = r.read()
            return body if raw else json.loads(body)
        except urllib.error.HTTPError as exc:
            if exc.code == 404:
                return None
            if exc.code != 429 or attempt == 3:
                raise
            time.sleep(5 * attempt)


def norm(text):
    return re.sub(r"[^a-z0-9]+", " ", (text or "").lower()).strip()


def surname(name):
    return (name.split(",")[0] if "," in name else name.split(" ")[-1]).strip()


def same_title(wanted, title, subtitle=None):
    w = norm(wanted)
    return bool(w) and w in {norm(title), norm(f"{title} {subtitle or ''}")}


# Crossref / doi.org


def csl(doi):
    return get(
        "https://doi.org/" + urllib.parse.quote(doi), "application/vnd.citationstyles.csl+json"
    )


def crossref_search(r, item_type):
    year = r.get("year")
    params = {"query.bibliographic": r["title"], "rows": 20}
    if r.get("authors"):
        params["query.author"] = surname(r["authors"][0])
    if year:
        params["filter"] = f"from-pub-date:{year - 1},until-pub-date:{year + 1}"
    found = get("https://api.crossref.org/works?" + urllib.parse.urlencode(params)) or {}
    candidates = []
    for x in found.get("message", {}).get("items", []):
        if item_type in WANTED and x.get("type") not in WANTED[item_type]:
            continue
        title = (x.get("title") or [""])[0]
        sub = (x.get("subtitle") or [None])[0]
        if not same_title(r["title"], title, sub):
            continue
        people = {norm(p.get("family")) for p in x.get("author", []) + x.get("editor", [])}
        if r.get("authors") and norm(surname(r["authors"][0])) not in people:
            continue
        issued = (x.get("issued", {}).get("date-parts") or [[None]])[0][0]
        if year and issued != year:
            continue
        candidates.append(x["DOI"])
    return candidates[0] if candidates else None


def crossref_by_isbn(isbn):
    found = (
        get(
            "https://api.crossref.org/works?"
            + urllib.parse.urlencode({"filter": f"isbn:{isbn}", "rows": 5})
        )
        or {}
    )
    items = found.get("message", {}).get("items", [])
    books = [x for x in items if x.get("type") in WANTED["book"]]
    return (books or items or [{}])[0].get("DOI")


def title_parts(c):
    title = c.get("title") if isinstance(c.get("title"), str) else (c.get("title") or [""])[0]
    sub = c.get("subtitle")
    return title, sub[0] if isinstance(sub, list) and sub else sub


def from_csl(c):
    item_type = CSL_TYPES.get(c.get("type"), "book")
    title = c.get("title") if isinstance(c.get("title"), str) else (c.get("title") or [""])[0]
    sub = c.get("subtitle")
    sub = sub[0] if isinstance(sub, list) and sub else sub
    parts = (c.get("issued", {}).get("date-parts") or [[None]])[0]
    fields = {
        "title": f"{title}: {sub}" if sub else title,
        "date": "-".join(f"{p:02d}" if i else str(p) for i, p in enumerate(parts) if p) or None,
        "publisher": c.get("publisher"),
        "place": c.get("publisher-place"),
        "edition": c.get("edition"),
        "ISBN": " ".join(dict.fromkeys(c.get("ISBN") or [])) or None,
        "DOI": c.get("DOI"),
        "url": f"https://doi.org/{c['DOI']}" if c.get("DOI") else None,
        "pages": c.get("page"),
        "volume": c.get("volume"),
        "issue": c.get("issue"),
        "numPages": c.get("number-of-pages"),
        "series": c.get("collection-title"),
        "seriesNumber": c.get("collection-number"),
        "language": c.get("language"),
        "ISSN": " ".join(c.get("ISSN") or []) or None,
        "libraryCatalog": "Crossref (via doi.org)",
    }
    container = c.get("container-title")
    container = container[0] if isinstance(container, list) and container else container
    if container:
        fields["bookTitle" if item_type == "bookSection" else "publicationTitle"] = container
    creators = [
        {"creatorType": role, "lastName": p.get("family", ""), "firstName": p.get("given", "")}
        if p.get("family")
        else {"creatorType": role, "name": p.get("literal") or p.get("name", "")}
        for role in ("author", "editor", "translator")
        for p in c.get(role, [])
    ]
    return item_type, clean(fields), creators


# K10plus


def k10plus(query):
    params = {
        "version": "1.1",
        "operation": "searchRetrieve",
        "query": query,
        "maximumRecords": 10,
        "recordSchema": "mods36",
    }
    body = get(K10PLUS + "?" + urllib.parse.urlencode(params), raw=True)
    if not body:
        return []
    return [mods_record(m) for m in ET.fromstring(body).iter("{http://www.loc.gov/mods/v3}mods")]


def mods_record(m):
    def texts(path):
        return [e.text.strip() for e in m.findall(path, MODS) if e.text and e.text.strip()]

    people, names = [], []
    for n in m.findall("m:name[@type='personal']", MODS):
        family = [p.text for p in n.findall("m:namePart[@type='family']", MODS)]
        given = [p.text for p in n.findall("m:namePart[@type='given']", MODS)]
        codes = [r.text for r in n.findall("m:role/m:roleTerm[@type='code']", MODS)]
        role = next((ROLES[c] for c in codes if c in ROLES), None)
        if family:
            names.append(family[0])
        if family and role:
            people.append(
                {"creatorType": role, "lastName": family[0], "firstName": (given or [""])[0]}
            )
    extent = texts("m:physicalDescription/m:extent")
    return {
        # Catalogues file a leading article apart ("A ", "The ") as nonSort.
        "title": " ".join(texts("m:titleInfo[1]/m:nonSort") + texts("m:titleInfo[1]/m:title")[:1]),
        "subtitle": (texts("m:titleInfo[1]/m:subTitle") or [None])[0],
        "creators": people,
        "names": names,  # every personal name, whatever its role (often unspecified)
        "place": (texts("m:originInfo/m:place/m:placeTerm[@type='text']") or [None])[0],
        "publisher": (texts("m:originInfo/m:publisher") or [None])[0],
        "date": (texts("m:originInfo/m:dateIssued") or [None])[0],
        "edition": (texts("m:originInfo/m:edition") or [None])[0],
        "isbns": [e.text for e in m.findall("m:identifier[@type='isbn']", MODS) if e.text],
        "doi": (texts("m:identifier[@type='doi']") or [None])[0],
        "extent": (extent or [None])[0],
        "online": any("online" in e.lower() for e in extent),
        "series": (texts("m:relatedItem[@type='series']/m:titleInfo/m:title") or [None])[0],
        "language": (texts("m:language/m:languageTerm[@type='code']") or [None])[0],
    }


def pick(records):
    """The print record (pages and size, not "Online-Ressource"), with creators from the
    record of the same edition that gives roles most fully (catalogues often omit them)."""
    print_ = [x for x in records if not x["online"] and x["isbns"]]
    best = (print_ or [x for x in records if x["isbns"]] or records or [None])[0]
    if best is not None:
        fullest = max(
            records,
            key=lambda x: (
                any(p["creatorType"] == "author" for p in x["creators"]),
                len(x["creators"]),
            ),
        )
        if len(fullest["creators"]) > len(best["creators"]) or not any(
            p["creatorType"] == "author" for p in best["creators"]
        ):
            best = {**best, "creators": fullest["creators"]}
    return best


def k10plus_search(r):
    clauses = [f'pica.tit="{r["title"]}"']
    if r.get("authors"):
        clauses.append("pica.per=" + surname(r["authors"][0]))
    if r.get("year"):
        clauses.append(f"pica.jhr={r['year']}")
    found = [
        x
        for x in k10plus(" and ".join(clauses))
        if same_title(r["title"], x["title"], x["subtitle"])
        and (not r.get("year") or (x["date"] or "")[:4] == str(r["year"]))
        and (
            not r.get("authors") or norm(surname(r["authors"][0])) in {norm(n) for n in x["names"]}
        )
    ]
    if r.get("publisher"):  # one edition, two publishers (e.g. US and UK issues)
        same = [x for x in found if norm(r["publisher"]) in norm(x["publisher"])]
        found = same or found
    return pick(found)


def k10plus_by_isbn(isbn):
    return pick(k10plus(f"pica.isb={isbn}"))


def pages(extent):
    """'XXI, 490 Seiten Illustrationen' -> 'XXI, 490'."""
    m = re.match(r"\s*((?:[IVXLC]+,\s*)?\d+)\s*(?:S\.|Seiten|p\.|pages)", extent or "")
    return m.group(1) if m else None


def from_mods(x):
    fields = {
        "title": f"{x['title']}: {x['subtitle']}" if x["subtitle"] else x["title"],
        "date": (x["date"] or "")[:4] or None,
        "publisher": x["publisher"],
        "place": x["place"],
        "edition": x["edition"],
        "ISBN": " ".join(dict.fromkeys(i for i in x["isbns"] if len(i) == 13) or x["isbns"])
        or None,
        "DOI": x["doi"],
        "numPages": pages(x["extent"]),
        "series": x["series"],
        "language": x["language"],
        "libraryCatalog": "K10plus",
    }
    return "book", clean(fields), x["creators"]


def clean(fields):
    if fields.get("place"):  # catalogue shorthand for "and other places"
        fields["place"] = re.sub(r"\s*\[(?:u\.\s*a\.|etc\.|et al\.)\]", "", fields["place"]).strip()
    return {k: v for k, v in fields.items() if v not in (None, "", [])}


# Resolution


def item_type_of(r):
    t = r.get("item_type")
    return {"chapter": "bookSection", "article": "journalArticle", "paper": "journalArticle"}.get(
        t, t
    )


def resolve(r):
    """Zotero item data for a request, or the reason none was accepted."""
    sources = []
    doi = r.get("doi")
    isbn = (re.findall(r"[0-9X]{10,13}", re.sub(r"-", "", r.get("isbn") or "")) or [None])[0]
    wanted = item_type_of(r)
    if not doi and isbn:
        doi = crossref_by_isbn(isbn)
        if doi:
            sources.append(f"Crossref: DOI {doi} for ISBN {isbn}")
    if not doi and not isbn:
        doi = crossref_search(r, wanted)
        if doi:
            sources.append(f"Crossref search: {doi} (same title, author, year)")
    record = None
    if doi:
        c = csl(doi)
        if c:
            item_type, fields, creators = from_csl(c)
            sources.append(f"doi.org CSL for {doi}")
            record = (item_type, fields, creators)
    if record is None:
        x = k10plus_by_isbn(isbn) if isbn else k10plus_search(r)
        if x:
            record = from_mods(x)
            sources.append(f"K10plus record (ISBN {', '.join(x['isbns'])})")
            if not record[1].get("DOI"):
                for candidate in x["isbns"]:
                    found = crossref_by_isbn(candidate)
                    c = csl(found) if found else None
                    if c and same_title(r["title"], *title_parts(c)):
                        record[1]["DOI"] = c["DOI"]
                        record[1]["url"] = f"https://doi.org/{c['DOI']}"
                        sources.append(f"Crossref: DOI {c['DOI']} for ISBN {candidate}")
                        break
    if record is None:
        return {
            "unresolved": "no record with the same title, author and year in Crossref or K10plus"
        }
    item_type, fields, creators = record
    if fields.get("libraryCatalog", "").startswith("Crossref") and fields.get("ISBN"):
        # Place, edition, page count and series are rarely in Crossref.
        for candidate in fields["ISBN"].split():
            x = k10plus_by_isbn(candidate)
            if x:
                _, extra, _ = from_mods(x)
                added = {
                    k: extra[k]
                    for k in ("place", "edition", "numPages", "series")
                    if k in extra and k not in fields and item_type == "book"
                } | {k: extra[k] for k in ("place",) if k in extra and k not in fields}
                if item_type == "bookSection" and not any(
                    p["creatorType"] == "editor" for p in creators
                ):
                    editors = [p for p in x["creators"] if p["creatorType"] == "editor"]
                    if editors:
                        creators = creators + editors
                        added["editors"] = True
                if added:
                    fields.update({k: v for k, v in added.items() if k != "editors"})
                    sources.append(f"K10plus ISBN {candidate}: {', '.join(sorted(added))}")
                break
    if r.get("edition") and "edition" not in fields:
        fields["edition"] = r["edition"]
        sources.append("edition from the request")
    return {"item_type": item_type, "fields": fields, "creators": creators, "sources": sources}
