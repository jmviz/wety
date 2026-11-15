import "./constellation.css";
import { Forest, TreeNode } from "../ety/model";
import { close as closePopover, hoverEnter, hoverLeave, pin, popover } from "../ety/popover";
import { useGoTo } from "../search/navigate";
import { Item, Lang, TreeKind, term } from "../search/types";
import { cognates, constellationStart, lastRequest, showGlosses, showRare } from "../state";
import { Branch, Common, Group, HIDE_UNDEFINED_ROOTS, LangTree, autoExpandGroup, branchesOf, canOpen, directEarlyEnglish, previewLines, previewRows, rackOrder, restrictBranches, singleEndpoint, sourceBranches, uncommonLatinEnglish, withoutUndefinedRoots } from "./branches";
import { BlobSpec, Layout, Link, Point, Rect, Size, SOURCE_HEIGHT, bounds, join, layout, rootDirection, wire } from "./layout";
import { chronological, languageTimes } from "./chronology";
import type { LayoutPeriod } from "./chronology";
import { representatives } from "./representatives";
import type { Frequencies } from "./representatives";
import { expansionDepth, sharedDepth, singleDescendantChildren, unbranchedLineage } from "./expansion";
import type { ExpansionDepth } from "./expansion";
import { compactBranches } from "./density";
import type { DensityNode } from "./density";

import {
  For,
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  on,
  onCleanup,
  onMount,
} from "solid-js";

// A word with its branches: the tree's root, drawn as a disc in the middle,
// or a head the reader opened, which is a row in the blob it was opened in
// (`via`) and has no shape of its own.
interface DisplayBranch extends Branch {
  origins?: Map<Group, Hub>;
}

interface Hub {
  id: string;
  node: TreeNode;
  parent: Hub | null;
  via: string | null;
  branches: DisplayBranch[];
  // Several sibling sources can feed the same language preview.
  shared: { host: Hub; branch: DisplayBranch }[];
  // The colour of the branch it was opened from, which its own branches keep.
  hue: string | null;
  hues: Map<string, string>;
  // A head with one word under it: the word stands alone beside the head's
  // row, named as the row is, in a blob as wide as the rack and as tall as
  // the row.
  single: boolean;
  // Rows can share a source, so each breakout keeps its own identity and
  // modern-word filter through every later language stage.
  scope?: { id: string; words: ReadonlySet<string> };
}

// One colour per branch of the root, as "r,g,b".
const PALETTE = [
  "125,211,252",
  "251,191,119",
  "196,181,253",
  "134,239,172",
  "249,168,212",
  "94,234,212",
  "253,164,145",
  "190,242,100",
];
const ROOT_R = 96;
const MIN_ZOOM = 0.35;
const MAX_ZOOM = 1.6;
// The first view keeps clear of the stage's edges and its corner text, and
// is laid out to fit at a zoom the words still read at.
const PAD_X = 48;
const PAD_Y = 96;
const READABLE = 0.85;

// A meaning short enough to set as large as its word: the first sense given
// ("to see" of "to see, perceive; look (at)"), without a trailing aside.
function brief(gloss: string): string {
  const first = gloss.split(";")[0];
  const sense = [...first].length > 22 ? first.split(",")[0] : first;
  return sense.replace(/\s*\([^)]*\)\s*$/, "").trim() || gloss;
}

const blobId = (hub: Hub, branch: Branch) => `${hub.id}/${branch.key}`;

// The branches met most often keep one colour whatever word was searched.
const FIXED: Record<string, number> = {
  "gem-pro": 0,
  "itc-pro": 1,
  "grk-pro": 2,
  "cel-pro": 3,
  "iir-pro": 4,
  "ine-bsl-pro": 5,
};

// A colour for each of a hub's branches: its fixed one, else whichever is
// still free, starting from one that depends only on the branch.
function huesOf(keys: string[]): Map<string, string> {
  const taken = new Set(keys.map((k) => FIXED[k]).filter((s) => s !== undefined));
  const hues = new Map<string, string>();
  for (const key of keys) {
    let slot = FIXED[key];
    if (slot === undefined) {
      let h = 0;
      for (const ch of key) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
      slot = h % PALETTE.length;
      for (let i = 0; i < PALETTE.length && taken.has(slot); i++) slot = (slot + 1) % PALETTE.length;
      taken.add(slot);
    }
    hues.set(key, PALETTE[slot]);
  }
  return hues;
}

// The words of the searched word's language that share its root, as a map:
// the root in the middle and round it a blob for each branch of the language
// tree they came down, listing the branch's words. Opening a blob shows the
// words of the branch they come from (its heads) as rows in it, each with
// its own branches' blobs beside it, and so on outwards.
export default function Constellation(props: { forest: Forest }) {
  const [data] = createResource(async () => {
    const [langs, words, dates] = await Promise.all([import("virtual:lang-tree"), import("virtual:common-words"), import("virtual:layout-periods")]);
    const common: Common = new Map();
    for (const [n, list] of Object.entries(words.levels)) for (const w of list.split("\n")) common.set(w, Number(n));
    const frequencies = new Map<string, number>();
    for (const [n, list] of Object.entries(words.frequencies)) for (const w of list.split("\n")) frequencies.set(w, Number(n) / 100);
    return { langs: langs as LangTree, common, periods: dates.periods, frequencies };
  });
  return (
    <Show
      when={props.forest.roots.length === 1}
      fallback={<p class="empty">The filters hide the word this constellation starts from.</p>}
    >
      <Show when={data()} fallback={<p class="empty" role="status">Loading languages…</p>}>
        {(d) => (
          // Drawn afresh when the display panel's "rare and derived words" is
          // switched: what a blob lists is settled when it is made.
          <Show
            when={showRare()}
            fallback={<Stage forest={props.forest} langs={d().langs} common={d().common} periods={d().periods} frequencies={d().frequencies} />}
          >
            <Stage forest={props.forest} langs={d().langs} common={null} periods={d().periods} frequencies={d().frequencies} />
          </Show>
        )}
      </Show>
    </Show>
  );
}

function Stage(props: { forest: Forest; langs: LangTree; common: Common | null; periods: LayoutPeriod[]; frequencies: Frequencies }) {
  let stageEl!: HTMLDivElement;
  const root = props.forest.roots[0];
  const target: Lang = (props.forest.selected ?? root).item.lang;
  // The word list is English: another language's words are all listed.
  const common = target.code === "en" ? props.common : null;
  const times = languageTimes(props.periods, props.langs, target.code, new Date().getFullYear());
  // Classical Latin ends around 200 CE in the shipped period registry.
  // Broad Ancient Greek and proto labels also count as early sources even
  // where their registry dates describe a later boundary or breakup.
  const omitted = target.code === "en" ? directEarlyEnglish(root, (code) =>
    code === "la" || code === "grc" || code.endsWith("-pro") || (times.get(code) ?? Infinity) <= 200
  ) : new Set<number>();
  for (const key of uncommonLatinEnglish(root, common)) omitted.add(key);
  const scores = target.code === "en" ? props.frequencies : undefined;
  const displayBranches = (node: TreeNode) => {
    const branches = branchesOf(node, target, props.langs, common, omitted, scores);
    return HIDE_UNDEFINED_ROOTS ? withoutUndefinedRoots(branches) : branches;
  };
  if (common && scores) {
    const preliminary = new WeakMap<TreeNode, Branch[]>();
    const read = (node: TreeNode): Branch[] => {
      let branches = preliminary.get(node);
      if (!branches) {
        branches = displayBranches(node);
        preliminary.set(node, branches);
      }
      return branches;
    };
    const groups = read(root).flatMap((b) => previewLines(b, target, read, common, scores, false));
    const keep = representatives(groups, scores);
    const visit = (n: TreeNode) => {
      if (n.item.lang.code === "en" && !n.isSelected && !keep.has(n.item.term)) omitted.add(n.key);
      n.children.forEach(visit);
    };
    visit(root);
  }

  const branchCache = new WeakMap<TreeNode, Branch[]>();
  const branchesFor = (node: TreeNode): Branch[] => {
    let branches = branchCache.get(node);
    if (!branches) {
      branches = displayBranches(node);
      branchCache.set(node, branches);
    }
    return branches;
  };
  const makeHub = (node: TreeNode, parent: Hub | null, via: string | null, hue: string | null, scope = parent?.scope): Hub => {
    const available = scope ? restrictBranches(branchesFor(node), scope.words) : branchesFor(node);
    const branches = available.map((b) => ({ ...b, groups: [...b.groups], words: [...b.words] }));
    const keys = branches.map((b) => b.key);
    return {
      id: scope ? `${node.item.id}@${scope.id}` : String(node.item.id),
      scope,
      node,
      parent,
      via,
      branches,
      shared: [],
      hue,
      hues: hue ? new Map(keys.map((k) => [k, hue])) : huesOf(keys),
      // Not a head with no name (branches.ts `unnamed`): the one word under
      // it is the only name its line has, and stays in its own branch's blob.
      single: !!parent && !!node.item.term && singleEndpoint(branches),
    };
  };
  const rootHub = makeHub(root, null, null, null);
  const rootWords = new Set(rootHub.branches.flatMap((b) => b.groups.flatMap((g) => g.words.map((w) => w.item.term)))).size;

  const [hubs, setHubs] = createSignal<Hub[]>([rootHub]);
  const [current, setCurrent] = createSignal<Hub>(rootHub);
  // A blob's size as drawn, and the room it holds on the canvas: the tallest
  // it has been. A blob that shrinks (a head opened, a list folded) keeps its
  // seat, so nothing round it moves.
  const [sizes, setSizes] = createSignal(new Map<string, { now: Size; seat: Size }>(), { equals: false });
  // In a rack, how far down its blob each head's row is centred: its line
  // leaves the blob there.
  const [rowMids, setRowMids] = createSignal(new Map<string, number>(), { equals: false });
  const [previewHeights, setPreviewHeights] = createSignal(new Map<string, number>(), { equals: false });

  const selective = (b: Branch) => b.groups.some((g) => canOpen(g, target));
  const previewCache = new WeakMap<Branch, ReturnType<typeof previewRows>>();
  const previewsFor = (b: Branch) => {
    let rows = previewCache.get(b);
    if (!rows) {
      rows = previewRows(b, target, branchesFor, common, scores);
      previewCache.set(b, rows);
    }
    return rows;
  };
  const rowKey = (hub: Hub, b: Branch, index: number) => `${blobId(hub, b)}:row:${index}`;
  const previewBoxId = (hub: Hub, b: Branch) => `${blobId(hub, b)}:preview`;
  const remainingRows = (hub: Hub, b: Branch, heads: Hub[]) => previewsFor(b).map((row, i) => ({ row, i }))
    .filter(({ row, i }) => !heads.some((h) => h.scope?.id === rowKey(hub, b, i)) &&
      !row.groups.filter((g) => canOpen(g, target)).every((g) => heads.some((h) => h.node.item.id === g.head.item.id
        && row.words.every((w) => h.scope?.words.has(w.item.term) ?? true))))
    .map(({ i }) => i);

  // Blobs are measured as drawn; the layout follows their real sizes.
  const observer = new ResizeObserver((entries) => {
    const map = sizes();
    const mids = rowMids();
    let changed = false;
    let moved = false;
    for (const entry of entries) {
      const el = entry.target as HTMLElement;
      if (el.dataset.previewBox) {
        const heights = previewHeights();
        if (heights.get(el.dataset.previewBox) !== el.offsetHeight) {
          heights.set(el.dataset.previewBox, el.offsetHeight);
          setPreviewHeights(heights);
        }
        continue;
      }
      const id = el.dataset.blob!;
      const now = { w: el.offsetWidth, h: el.offsetHeight };
      const had = map.get(id);
      if (now.w && now.h && (!had || had.now.w !== now.w || had.now.h !== now.h)) {
        // A proto column's height comes from the layout, not its contents;
        // do not retain that virtual height when it returns to a preview.
        map.set(id, { now, seat: { w: now.w, h: el.classList.contains("clustered")
          ? SOURCE_HEIGHT : Math.max(now.h, had?.seat.h ?? 0) } });
        changed = true;
      }
      for (const row of el.querySelectorAll<HTMLElement>("[data-row]")) {
        const mid = row.offsetTop + row.offsetHeight / 2;
        if (mids.get(row.dataset.row!) !== mid) {
          mids.set(row.dataset.row!, mid);
          moved = true;
        }
      }
    }
    if (changed) setSizes(map);
    if (moved) setRowMids(mids);
  });
  onCleanup(() => observer.disconnect());

  // The room the first view has: the stage as first seen, at a zoom the
  // words still read at. Fixed from then on, so resizing the window does not
  // shuffle the blobs.
  const [frame, setFrame] = createSignal<Size>();
  const [availableWidth, setAvailableWidth] = createSignal(1000);
  const [layoutVersion, setLayoutVersion] = createSignal(0);
  let placedVersion = 0;
  const clusteredHeads = (heads: Hub[]) => heads.length > 1 && heads.every((h) => h.node.item.lang.code.endsWith("-pro"));
  const clusteredBranch = (b: Branch, heads: Hub[]) => clusteredHeads(heads) || (selective(b) && heads.length > 0);
  const entriesFor = (h: Hub, b: Branch, heads: Hub[]): { id: string; head: Hub | undefined }[] => selective(b) && heads.length > 0
    ? [
        ...(remainingRows(h, b, heads).length ? [{ id: previewBoxId(h, b), head: undefined }] : []),
        ...heads.map((head) => ({ id: head.id, head })),
      ]
    : heads.map((head) => ({ id: head.id, head }));
  // A crowded family compresses all of its individual source cards,
  // including short side branches, while preserving the preview boxes.
  const chainIds = createMemo(() => {
    const all = hubs();
    const densityNode = (h: Hub, b: Branch): DensityNode => {
      const id = blobId(h, b);
      const open = all.filter((k) => k.via === id);
      const source = b.groups.length === 1 && open.length === 1 && !!open[0].node.item.term
        && remainingRows(h, b, open).length === 0;
      const endpoint = b.groups.every((g) => g.own) && b.words.length === 1;
      return { id, width: open.length ? 320 : b.words.length <= 4 ? 250 : 320,
        compact: source || endpoint,
        children: open.flatMap((child) => child.branches.map((next) => densityNode(child, next))) };
    };
    return compactBranches(rootHub.branches.map((b) => densityNode(rootHub, b)), availableWidth());
  });
  // What the layout is given: the root's blobs and, in each blob that has
  // been opened, its heads, each with its own blobs in turn.
  const spec = createMemo(() => {
    const measured = sizes();
    const mids = rowMids();
    const all = hubs();
    const blobsOf = (h: Hub, compactLanes = false): BlobSpec[] =>
      h.branches.map((b) => {
        const id = blobId(h, b);
        const heads = all.filter((k) => k.via === id);
        const clustered = clusteredBranch(b, heads);
        const terminal = h.single || b.groups.every((g) => g.own);
        const stage = terminal ? target.code : (b.language ?? b.key);
        return {
          id,
          terminal,
          stage,
          year: times.get(stage),
          clustered,
          compactChain: chainIds().has(id),
          // A rack is laid out at the size it is now, so its rows' lines and
          // the words level with them leave it where the rows are drawn; a
          // list keeps the room it has had. Proto clusters use current child
          // sizes so removed internal headings leave no empty space behind.
          size: clustered && measured.has(id) ? { w: measured.get(id)!.now.w, h: SOURCE_HEIGHT }
            : heads.length || compactLanes ? measured.get(id)?.now : measured.get(id)?.seat,
          lanes: heads.length
            ? entriesFor(h, b, heads).map((entry) => entry.head
              ? { hub: entry.id, mid: mids.get(entry.id), blobs: blobsOf(entry.head, compactLanes || clustered), inline: entry.head.single }
              : { hub: entry.id, mid: undefined, height: previewHeights().get(entry.id) ?? 64, blobs: [] })
            : undefined,
        };
      });
    return { id: rootHub.id, r: ROOT_R, blobs: blobsOf(rootHub).map(chronological) };
  });
  const placed = createMemo<Layout>(
    (before) => {
      const room = frame();
      const version = layoutVersion();
      const previous = version === placedVersion ? before : undefined;
      placedVersion = version;
      return room ? layout(spec(), room, previous) : before;
    },
    { hubs: new Map(), blobs: new Map(), seats: new Map(), sides: new Map(), sources: new Map() }
  );
  // Where a blob is drawn: its seat, or when it has shrunk, the end of its
  // seat nearer its hub.
  const drawn = (hub: Hub, id: string): Rect | undefined => {
    const seat = placed().blobs.get(id);
    const now = sizes().get(id)?.now;
    const c = placed().hubs.get(hub.id);
    if (hubs().some((h) => h.via === id && placed().sources.has(h.id))) return seat;
    if (!seat || !now || !c || now.h >= seat.h) return seat;
    const below = c.y > seat.y + seat.h / 2;
    return { x: seat.x, y: below ? seat.y + seat.h - now.h : seat.y, w: seat.w, h: now.h };
  };

  // A set of source roots shares one language heading above its column.
  const languageKey = (h: Hub, b: Branch) => `${h.hue}/${b.language ?? b.key}`;
  const languageLabels = createMemo(() => {
    const groups = new Map<string, { name: string; hue: string; boxes: Rect[]; external: boolean }>();
    for (const h of hubs()) for (const b of h.branches) {
      const open = hubs().filter((k) => k.via === blobId(h, b));
      const clustered = clusteredBranch(b, open);
      if (!h.parent && !clustered) continue;
      const box = drawn(h, blobId(h, b));
      if (!box) continue;
      const compact = h.single || (b.groups.length === 1 && b.groups[0].own && b.words.length === 1) ||
        (b.groups.length === 1 && open.length === 1 && !!open[0].node.item.term);
      if (!open.length && !compact) continue;
      const key = languageKey(h, b);
      let group = groups.get(key);
      if (!group) groups.set(key, (group = { name: b.name, hue: h.hues.get(b.key)!, boxes: [], external: false }));
      group.boxes.push(...(clustered ? open.flatMap((k) => placed().sources.get(k.id) ?? []) : [box]));
      group.external ||= compact || clustered;
    }
    return new Map([...groups].filter(([, g]) => g.boxes.length && (g.boxes.length > 1 || g.external)).map(([key, g]) => [key, {
      name: g.name, hue: g.hue, x: Math.min(...g.boxes.map((b) => b.x)), y: Math.min(...g.boxes.map((b) => b.y)) - 24,
    }]));
  });

  // One string per line, so a line that has not moved keeps its element.
  const links = createMemo(() => {
    const at = placed();
    const out: string[] = [];
    const push = (hue: string, l: Link) =>
      out.push(
        [hue, l.d, l.end.x.toFixed(1), l.end.y.toFixed(1), l.start?.x.toFixed(1) ?? "", l.start?.y.toFixed(1) ?? ""].join("|")
      );
    for (const h of hubs()) {
      const c = at.hubs.get(h.id);
      if (!c) continue;
      // From the root's rim to each blob round it; from a head's row to the
      // near side of each blob in its lane.
      const side = h.via ? (at.sides.get(h.via) ?? 1) : 0;
      for (const shared of h.shared) {
        const box = drawn(shared.host, blobId(shared.host, shared.branch));
        const sources = entriesFor(shared.host, shared.branch, hubs().filter((k) => k.via === blobId(shared.host, shared.branch)))
          .flatMap((k) => at.sources.get(k.id) ?? []);
        const ys = sources.map((s) => s.y + s.h / 2);
        const open = ys.length > 0;
        if (box && side) push(h.hue!, wire(c, {
          x: side > 0 ? box.x - (open ? 24 : 0) : box.x + box.w + (open ? 24 : 0),
          y: open ? Math.max(Math.min(...ys), Math.min(Math.max(...ys), c.y)) : box.y + box.h / 2,
        }));
      }
      for (const b of h.branches) {
        const box = drawn(h, blobId(h, b));
        if (!box) continue;
        const hue = h.hues.get(b.key)!;
        const sources = entriesFor(h, b, hubs().filter((k) => k.via === blobId(h, b))).flatMap((k) => at.sources.get(k.id) ?? []);
        const outward = at.sides.get(blobId(h, b)) ?? (side || (box.x + box.w / 2 >= c.x ? 1 : -1));
        const spineX = outward > 0 ? box.x - 24 : box.x + box.w + 24;
        const ys = sources.map((s) => s.y + s.h / 2);
        const arrival = sources.length ? { x: spineX, y: Math.max(Math.min(...ys), Math.min(Math.max(...ys), c.y)), w: 0, h: 0 } : box;
        if (side) push(hue, wire(c, { x: side > 0 ? arrival.x : arrival.x + arrival.w, y: arrival.y + arrival.h / 2 }));
        else push(hue, join(c, ROOT_R, arrival,
          [...at.seats].filter(([id]) => id !== blobId(h, b)).map(([, seat]) => seat).concat(sources.length ? [box] : []),
          rootDirection(h.branches.indexOf(b), h.branches.length)
        ));
        if (sources.length) {
          push(hue, { d: `M ${spineX} ${Math.min(...ys)} L ${spineX} ${Math.max(...ys)}`,
            end: { x: spineX, y: arrival.y + arrival.h / 2 } });
          for (const source of sources) {
            const y = source.y + source.h / 2;
            push(hue, wire({ x: spineX, y }, { x: outward > 0 ? source.x : source.x + source.w, y }));
          }
        }
      }
    }
    return out;
  });

  // The view: where the canvas origin sits in the stage, and the zoom.
  const [view, setView] = createSignal({ x: 0, y: 0, k: 1 });
  const [stage, setStage] = createSignal<Size>({ w: 0, h: 0 });
  const [gliding, setGliding] = createSignal(false);
  const [dragging, setDragging] = createSignal(false);
  let glideTimer: number | undefined;
  const glide = (next: { x: number; y: number; k: number }) => {
    setGliding(true);
    setView(next);
    window.clearTimeout(glideTimer);
    glideTimer = window.setTimeout(() => setGliding(false), 700);
  };
  const centreOn = (p: Point) => {
    const { k } = view();
    glide({ x: stage().w / 2 - p.x * k, y: stage().h / 2 - p.y * k, k });
  };
  const zoomAt = (cx: number, cy: number, k: number, smooth = false) => {
    const v = view();
    const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, k));
    const to = { x: cx - ((cx - v.x) / v.k) * next, y: cy - ((cy - v.y) / v.k) * next, k: next };
    if (smooth) glide(to);
    else setView(to);
  };

  const centreOnAll = (rects: Rect[]) => {
    if (!rects.length) return;
    const x0 = Math.min(...rects.map((b) => b.x));
    const x1 = Math.max(...rects.map((b) => b.x + b.w));
    const y0 = Math.min(...rects.map((b) => b.y));
    const y1 = Math.max(...rects.map((b) => b.y + b.h));
    centreOn({ x: (x0 + x1) / 2, y: (y0 + y1) / 2 });
  };
  // Bring a rack into view whole: centred, and zoomed out as far as it takes
  // (never in), so its lines can be followed from end to end.
  const fitAll = (rects: Rect[]) => {
    const s = stage();
    if (!rects.length || !s.w) return;
    const x0 = Math.min(...rects.map((b) => b.x));
    const x1 = Math.max(...rects.map((b) => b.x + b.w));
    const y0 = Math.min(...rects.map((b) => b.y));
    const y1 = Math.max(...rects.map((b) => b.y + b.h));
    const k = Math.max(MIN_ZOOM, Math.min(view().k, (s.w - PAD_X) / (x1 - x0), (s.h - PAD_Y) / (y1 - y0)));
    glide({ x: s.w / 2 - ((x0 + x1) / 2) * k, y: s.h / 2 - ((y0 + y1) / 2) * k, k });
  };
  // A blob with everything racked off it.
  const rackRects = (hub: Hub, id: string): Rect[] => {
    const box = drawn(hub, id);
    return [
      ...(box ? [box] : []),
      ...hubs()
        .filter((k) => k.via === id)
        .flatMap((k) => k.branches.flatMap((b) => rackRects(k, blobId(k, b)))),
    ];
  };
  // Centre on a hub with what comes off it: the root's disc and its blobs,
  // or the blob a head is a row of and the head's own blobs.
  const centreOnHub = (hub: Hub) => {
    const c = placed().hubs.get(hub.id);
    const body = hub.parent && hub.via ? drawn(hub.parent, hub.via) : c && { x: c.x - ROOT_R, y: c.y - ROOT_R, w: ROOT_R * 2, h: ROOT_R * 2 };
    centreOnAll([...(body ? [body] : []), ...hub.branches.flatMap((b) => rackRects(hub, blobId(hub, b)))]);
  };
  // What has just been opened is measured and placed over the next few
  // frames: centre on it once it has settled.
  const settled = (then: () => void) => {
    let frames = 4;
    const tick = () => (--frames ? requestAnimationFrame(tick) : then());
    requestAnimationFrame(tick);
  };
  const focus = (hub: Hub) => {
    setCurrent(hub);
    centreOnHub(hub);
  };
  // Open heads in a blob, all those given at once: the blob becomes a rack,
  // each head a row in it with its own blobs in a lane beside it. A blob
  // with one head is a rack of one row.
  const rack = (hub: Hub, branch: Branch, groups: Group[], scope = hub.scope) => {
    const id = blobId(hub, branch);
    const all = [...hubs()];
    const taken = new Set(all.map((h) => h.id));
    const peers = (owner: Hub, b: Branch) => all.flatMap((h) => h.branches
      .filter((k) => languageKey(h, k) === languageKey(owner, b))
      .map((k) => ({ owner: h, branch: k, id: blobId(h, k) })));
    const inherited = sharedDepth(peers(hub, branch).filter((p) => p.id !== id).map((p) => expansionDepth(p.id, all)));
    const combine = (made: Hub[]) => {
      const columns = new Map<string, { host: Hub; branch: DisplayBranch }>();
      for (const child of made) child.branches = child.branches.filter((b) => {
        // Actual target-language endpoints stay separate. Preview columns
        // combine only sibling sources, never unrelated family branches.
        if (b.groups.every((g) => g.own)) return true;
        const key = b.language ?? b.key;
        const previous = columns.get(key);
        if (!previous) {
          b.origins = new Map(b.groups.map((g) => [g, child]));
          columns.set(key, { host: child, branch: b });
          return true;
        }
        const combined = previous.branch;
        for (const g of b.groups) combined.origins!.set(g, child);
        combined.groups.push(...b.groups);
        const terms = new Set(combined.words.map((w) => w.item.term));
        for (const word of b.words) if (!terms.has(word.item.term)) {
          terms.add(word.item.term);
          combined.words.push(word);
        }
        combined.wordCount = combined.words.length;
        combined.hasSelected ||= b.hasSelected;
        child.shared.push(previous);
        return false;
      });
    };
    const expand = (owner: Hub, b: Branch, desired: ExpansionDepth | null, firstGroups = b.groups, selectedScope = owner.scope) => {
      const lineage = unbranchedLineage({ ...b, groups: firstGroups }, (head) => {
        const branches = branchesFor(head);
        return selectedScope ? restrictBranches(branches, selectedScope.words) : branches;
      });
      const queue: { owner: Hub; branch: Branch; level: number; groups: Group[]; automatic: boolean }[] = [
        { owner, branch: b, level: 1, groups: firstGroups, automatic: lineage || !!autoExpandGroup(b, target) },
      ];
      for (const step of queue) {
        const via = blobId(step.owner, step.branch);
        const existing = all.filter((h) => h.via === via && (!selectedScope || h.scope?.id === selectedScope.id));
        const made: Hub[] = [];
        for (const g of step.groups) {
          if (!canOpen(g, target)) continue;
          const origins = (step.branch as DisplayBranch).origins;
          const source = origins && [...origins].find(([group]) => group.head === g.head)?.[1];
          const child = makeHub(g.head, source ?? step.owner, via, step.owner.hues.get(step.branch.key)!, selectedScope);
          const key = child.id;
          if (taken.has(key)) continue;
          taken.add(key);
          made.push(child);
        }
        const children = [...existing, ...rackOrder(made, (h) => h.single)];
        const automatic = step.automatic || (step.level === 1 && singleDescendantChildren(children.map((h) => ({ branches: sourceBranches(h.branches) }))));
        combine(made);
        all.push(...rackOrder(made, (h) => h.single));
        for (const child of children) for (const next of child.branches) {
          // A merged preview is still a frontier until the reader chooses
          // a row, unless the established sibling horizon requires more.
          const follow = desired ? desired.complete || step.level < desired.depth
            : lineage || (automatic && sourceBranches([next]).every((b) => !!autoExpandGroup(b, target)));
          if (follow) queue.push({ owner: child, branch: next, level: step.level + 1, groups: next.groups, automatic });
        }
      }
    };
    expand(hub, branch, inherited, groups, scope);
    // Deepening an already-open path also advances its opened siblings.
    // Merely revealed/folded siblings never join the expansion.
    for (let source: Hub | null = hub; source?.parent && source.via; source = source.parent) {
      const parent = source.parent;
      const via = source.via;
      const b = parent.branches.find((b) => blobId(parent, b) === via);
      if (!b) continue;
      const openedPeers = peers(parent, b).filter((p) => expansionDepth(p.id, all).depth > 0);
      if (openedPeers.length < 2) continue;
      const horizon = sharedDepth(openedPeers.map((p) => expansionDepth(p.id, all)));
      if (horizon) for (const p of openedPeers) expand(p.owner, p.branch, horizon);
    }
    setHubs(all);
    closePopover();
    setCurrent(hub);
    settled(() => fitAll(rackRects(hub, id)));
  };
  // Back to one list: everything opened from the blob goes.
  const unrack = (hub: Hub, id: string) => {
    const gone = new Set<Hub>();
    for (const h of hubs()) if (h.via === id || (h.parent && gone.has(h.parent))) gone.add(h);
    closePopover();
    setHubs(hubs().filter((h) => !gone.has(h)));
    setCurrent(hub);
    settled(() => centreOnAll(rackRects(hub, id)));
  };
  // Lit: the current hub's blobs, and those racked off them.
  const lit = (hub: Hub): boolean => hub === current() || (!!hub.parent && lit(hub.parent));
  // The root and its branches, whole, as large as the stage allows.
  const rootView = () => {
    const b = bounds(placed(), rootHub.id, ROOT_R, rootHub.branches.map((br) => blobId(rootHub, br)));
    const s = stage();
    if (!b || !s.w) return null;
    const k = Math.min(1, Math.max(MIN_ZOOM, Math.min((s.w - PAD_X) / b.w, (s.h - PAD_Y) / b.h)));
    return { x: s.w / 2 - (b.x + b.w / 2) * k, y: s.h / 2 + 8 - (b.y + b.h / 2) * k, k };
  };
  // The first view: once, when every branch of the root has its place.
  let fitted = false;
  createEffect(() => {
    if (fitted || !rootHub.branches.every((b) => placed().blobs.has(blobId(rootHub, b)))) return;
    const v = rootView();
    if (!v) return;
    fitted = true;
    setView(v);
  });
  const startOver = () => {
    closePopover();
    setHubs([rootHub]);
    setCurrent(rootHub);
    settled(() => {
      // Forget space reserved by expanded racks and fit the original view.
      setSizes(new Map([...sizes()].map(([id, size]) => [id, { now: size.now, seat: size.now }])));
      setLayoutVersion((v) => v + 1);
      const v = rootView();
      if (v) glide(v);
    });
  };
  // Other ancestors the map could start from: the word's ultimate roots,
  // and the oldest form that is still the same word.
  const goTo = useGoTo();
  const starts = createMemo(() => {
    const c = cognates();
    const seen = new Set<number>();
    const out: Item[] = [];
    for (const it of c ? [...c.ultimateRoots, c.trail[c.trail.length - 1]?.item] : [])
      if (it && !seen.has(it.id)) {
        seen.add(it.id);
        out.push(it);
      }
    return out;
  });
  const startFrom = (it: Item) => {
    const c = cognates();
    const r = lastRequest();
    if (!c || !r || it.id === root.item.id) return;
    const usual = constellationStart(c)?.id ?? c.defaultRoot;
    goTo(TreeKind.Constellation, r.item, {}, it.id === usual ? null : it);
  };
  // The hubs from the root to the current one.
  const trail = () => {
    const out: Hub[] = [];
    for (let h: Hub | null = current(); h; h = h.parent) out.unshift(h);
    return out;
  };

  onMount(() => {
    // The stage takes what is left of the window below the word's title.
    const fill = () =>
      stageEl.style.setProperty("--cst-top", `${Math.round(stageEl.getBoundingClientRect().top + window.scrollY)}px`);
    fill();
    window.addEventListener("resize", fill);
    onCleanup(() => window.removeEventListener("resize", fill));
    const sized = new ResizeObserver(() => {
      const was = stage();
      const now = { w: stageEl.clientWidth, h: stageEl.clientHeight };
      if (!now.w || !now.h) return;
      setAvailableWidth((now.w - PAD_X) / READABLE);
      if (!was.w) {
        setView({ x: now.w / 2, y: now.h / 2, k: 1 });
        setFrame({ w: (now.w - PAD_X) / READABLE, h: (now.h - PAD_Y) / READABLE });
      } else {
        const v = view();
        setView({ ...v, x: v.x + (now.w - was.w) / 2, y: v.y + (now.h - was.h) / 2 });
      }
      setStage(now);
    });
    sized.observe(stageEl);
    // Not passive: the wheel moves the canvas, not the page.
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      if (!popover()?.pinned) closePopover();
      const v = view();
      if (e.ctrlKey || e.metaKey) {
        const r = stageEl.getBoundingClientRect();
        zoomAt(e.clientX - r.left, e.clientY - r.top, v.k * Math.exp(-e.deltaY * 0.01));
      } else {
        const unit = e.deltaMode === 1 ? 32 : 1;
        setGliding(false);
        setView({ ...v, x: v.x - e.deltaX * unit, y: v.y - e.deltaY * unit });
      }
    };
    stageEl.addEventListener("wheel", onWheel, { passive: false });
    stageEl.addEventListener("click", dropDragClick, true);
    onCleanup(() => {
      sized.disconnect();
      stageEl.removeEventListener("wheel", onWheel);
      stageEl.removeEventListener("click", dropDragClick, true);
      window.clearTimeout(glideTimer);
    });
  });

  // Dragging moves the canvas. The pointer is captured only once it has
  // moved, so a plain click still reaches the word under it.
  let drag: { id: number; x: number; y: number; vx: number; vy: number; moved: boolean } | null = null;
  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0 || (e.target as Element).closest(".cst-ui")) return;
    drag = { id: e.pointerId, x: e.clientX, y: e.clientY, vx: view().x, vy: view().y, moved: false };
  };
  const onPointerMove = (e: PointerEvent) => {
    if (!drag || e.pointerId !== drag.id) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (!drag.moved) {
      if (Math.hypot(dx, dy) < 5) return;
      drag.moved = true;
      stageEl.setPointerCapture(e.pointerId);
      closePopover();
      setGliding(false);
      setDragging(true);
    }
    setView({ ...view(), x: drag.vx + dx, y: drag.vy + dy });
  };
  // The click that ends a drag is not a click on what the drag began over.
  let dragged = false;
  const onPointerUp = (e: PointerEvent) => {
    if (drag?.id !== e.pointerId) return;
    if (drag.moved) {
      dragged = true;
      window.setTimeout(() => (dragged = false), 0);
    }
    drag = null;
    setDragging(false);
  };
  const dropDragClick = (e: MouseEvent) => {
    if (!dragged) return;
    e.stopPropagation();
    e.preventDefault();
  };

  // A word reached with the keyboard is brought into view.
  const onFocusIn = (e: FocusEvent) => {
    stageEl.scrollLeft = 0;
    stageEl.scrollTop = 0;
    const el = e.target as HTMLElement;
    if (!el.matches(":focus-visible") || el.closest(".cst-ui")) return;
    const r = el.getBoundingClientRect();
    const s = stageEl.getBoundingClientRect();
    const m = 56;
    const dx = r.left < s.left + m ? s.left + m - r.left : r.right > s.right - m ? s.right - m - r.right : 0;
    const dy = r.top < s.top + m ? s.top + m - r.top : r.bottom > s.bottom - m ? s.bottom - m - r.bottom : 0;
    if (dx || dy) glide({ ...view(), x: view().x + dx, y: view().y + dy });
  };

  // Word details, as in the trees: hover to peek, click to pin. Where a whole
  // blob or root is one thing to click, its words are not buttons and a
  // click on one is a click on the blob or root.
  const WORD = ".cst-word, .cst-root";
  const wordFromEvent = (e: Event): [TreeNode, HTMLElement] | null => {
    const el = (e.target as Element).closest<HTMLElement>(WORD);
    const node = el && props.forest.byKey.get(Number(el.dataset.key));
    return el && node ? [node, el] : null;
  };
  const onClick = (e: MouseEvent) => {
    const hit = wordFromEvent(e);
    if (hit && hit[1].tagName === "BUTTON") pin(hit[0], hit[1]);
  };
  const onPointerOver = (e: PointerEvent) => {
    const hit = wordFromEvent(e);
    if (hit && !dragging()) hoverEnter(hit[0], hit[1]);
  };
  const onPointerOut = (e: PointerEvent) => {
    const el = (e.target as Element).closest(WORD);
    if (el && !el.contains(e.relatedTarget as Node)) hoverLeave();
  };

  // Words, comma separated. `live`: each is a button for its details.
  const WordList = (p: { words: TreeNode[]; live: boolean }) => (
    <For each={p.words}>
      {(w, i) => (
        <>
          {/* The comma stays on its word's line. */}
          <span class="cst-entry">
            <Show
              when={p.live}
              fallback={
                <span class="cst-word" classList={{ you: w.isSelected }} data-key={w.key}>
                  {term(w.item)}
                </span>
              }
            >
              <button
                type="button"
                class="cst-word"
                classList={{ you: w.isSelected }}
                data-key={w.key}
                aria-haspopup="dialog"
              >
                {term(w.item)}
              </button>
            </Show>
            {i() < p.words.length - 1 ? "," : ""}
          </span>{" "}
        </>
      )}
    </For>
  );

  // Language labels stand above their entries, leaving the row for the
  // word, its reading and meaning.
  const HeadName = (p: { item: Item }) => (
    <>
      <span class="cst-head-term">{term(p.item) || "Undefined Root"}</span>
      <Show when={p.item.romanization}>
        <span class="cst-rom">{p.item.romanization}</span>
      </Show>
      <Show when={showGlosses() && p.item.gloss?.[0]}>
        <span class="cst-gloss" title={p.item.gloss![0]}>
          {brief(p.item.gloss![0])}
        </span>
      </Show>
    </>
  );

  // A head that cannot be opened (its words are all there is under it), with
  // its words, each a button for its details.
  const GroupView = (p: { hub: Hub; branch: Branch; group: Group }) => {
    const item = p.group.head.item;
    return (
      <div class="cst-group">
        <Show when={!p.group.own}>
          <span class="cst-head static">
            <HeadName item={item} />
          </span>
        </Show>
        <p class="cst-words">
          <WordList words={p.group.words} live />
        </p>
      </div>
    );
  };

  const PreviewRow = (p: { hub: Hub; branch: Branch; index: number }) => {
    const row = previewsFor(p.branch)[p.index];
    const id = rowKey(p.hub, p.branch, p.index);
    return (
      <button
        type="button"
        class="cst-preview-row"
        data-row-choice={id}
        aria-expanded="false"
        aria-label={`Expand ${row.words.map((w) => w.item.term).join(", ")}`}
        disabled={!row.groups.some((g) => canOpen(g, target))}
        onClick={() => rack(p.hub, p.branch, row.groups, { id, words: new Set(row.words.map((w) => w.item.term)) })}
      >
        <span class="cst-words">
          <For each={row.lines}>{(words) => <span class="cst-preview-line"><WordList words={words} live={false} /></span>}</For>
        </span>
      </button>
    );
  };

  const PreviewBox = (p: { hub: Hub; branch: Branch; floating?: boolean }) => {
    let el!: HTMLDivElement;
    const id = previewBoxId(p.hub, p.branch);
    const remaining = () => remainingRows(p.hub, p.branch, hubs().filter((h) => h.via === blobId(p.hub, p.branch)));
    onMount(() => observer.observe(el));
    onCleanup(() => observer.unobserve(el));
    return (
      <div ref={el} class="cst-row-face" classList={{ "cst-preview-box": p.floating }} data-preview-box={id}
        style={p.floating ? { top: `${(placed().sources.get(id)?.y ?? 0) - (drawn(p.hub, blobId(p.hub, p.branch))?.y ?? 0)}px` } : undefined}>
        <span class="cst-fam">{p.branch.name} Descendants</span>
        <For each={remaining()}>{(index) => <PreviewRow hub={p.hub} branch={p.branch} index={index} />}</For>
      </div>
    );
  };

  const BlobView = (p: { hub: Hub; branch: Branch }) => {
    let el!: HTMLElement;
    const id = blobId(p.hub, p.branch);
    const rect = () => drawn(p.hub, id);
    // Unfolds from its hub: drawn there first, then moved to its place.
    const [shown, setShown] = createSignal(false);
    createEffect(() => {
      if (rect() && !shown()) requestAnimationFrame(() => setShown(true));
    });
    onMount(() => {
      observer.observe(el);
      onCleanup(() => {
        observer.unobserve(el);
        sizes().delete(id);
      });
    });
    const at = (): Point => {
      const r = rect();
      if (shown() && r) return r;
      const c = placed().hubs.get(p.hub.id) ?? { x: 0, y: 0 };
      return { x: c.x - 170, y: c.y - 60 };
    };
    // Proto previews open one clicked row at a time, preserving the box
    // around the untouched rows. Other branches open their heads together.
    const roots = p.branch.groups;
    // The heads opened here, in the order of their rows.
    const rows = () => hubs().filter((h) => h.via === id);
    const openable = roots.filter((g) => canOpen(g, target));
    const lines = () => previewLines(p.branch, target, branchesFor, common, scores);
    // One named word needs one row, even when more language stages follow.
    // Keeping those stages visible must not add a separate language header.
    const compact = () => roots.length === 1 && rows().length === 1 && !!rows()[0].node.item.term;
    const clustered = () => clusteredBranch(p.branch, rows());
    const face = () => openable.length > 0 && rows().length === 0;
    const act = () => rack(p.hub, p.branch, openable);
    const Name = () => <span>{p.branch.name}</span>;
    const sharedLanguage = () => languageLabels().has(languageKey(p.hub, p.branch));
    // A head's one word: set as the head's row is, and a button for its
    // details.
    if (p.hub.single || (roots.length === 1 && roots[0].own && p.branch.words.length === 1)) {
      const word = p.branch.words[0];
      return (
        <section
          ref={el}
          class="cst-blob single"
          classList={{ shown: shown() && !!rect(), dim: !lit(p.hub), "compact-chain": chainIds().has(id) }}
          data-blob={id}
          data-language={p.branch.name}
          aria-label={`${word.item.lang.name}: ${term(word.item)}`}
          style={{ "--c": p.hub.hues.get(p.branch.key), left: `${at().x}px`, top: `${at().y}px` }}
        >
          <button
            type="button"
            class="cst-root cst-head open"
            classList={{ you: word.isSelected }}
            data-key={word.key}
            aria-haspopup="dialog"
          >
            <span class="cst-head-term">{term(word.item)}</span>
          </button>
        </section>
      );
    }
    return (
      <section
        ref={el}
        class="cst-blob"
        classList={{
          shown: shown() && !!rect(),
          dim: !lit(p.hub) && current().via !== id,
          // A few words need less room; a head's row needs the full width.
          narrow: p.branch.wordCount <= 4 && rows().length === 0,
          // Its lanes run left: the rows stand against that edge.
          left: placed().sides.get(id) === -1,
          single: compact(),
          "has-close": rows().length > 0,
          clustered: clustered(),
          "compact-chain": chainIds().has(id),
        }}
        data-blob={id}
        data-language={p.branch.name}
        style={{ "--c": p.hub.hues.get(p.branch.key), left: `${at().x}px`, top: `${at().y}px`,
          "--cst-first-source": clustered() ? `${(placed().sources.get(rows()[0]?.id)?.y ?? at().y) - at().y}px` : undefined,
          height: clustered() ? `${rect()?.h ?? SOURCE_HEIGHT}px` : undefined }}
        aria-label={`${p.branch.name}: ${p.branch.wordCount} ${target.name} words`}
      >
        <Show when={rows().length > 0}>
          <button
            type="button"
            class="cst-close"
            aria-label={`Collapse ${p.branch.name}`}
            title="Collapse this node"
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              unrack(p.hub, id);
              el.querySelector<HTMLButtonElement>(".cst-face")?.focus({ preventScroll: true });
            }}
          />
        </Show>
        <Show
          when={face()}
          fallback={
            <div class="cst-body">
              <Show when={!compact() && !sharedLanguage() && !clustered()}>
                <Show
                  when={rows().length > 0}
                  fallback={
                    <header class="cst-fam">
                      <Name />
                    </header>
                  }
                >
                  <button
                    type="button"
                    class="cst-fam"
                    aria-expanded="true"
                    title="fold back into one list"
                    onClick={() => unrack(p.hub, id)}
                  >
                    <Name />
                  </button>
                </Show>
              </Show>
              <For each={rows()}>
                {(h) => (
                  <div class="cst-row" data-row={h.id} style={clustered() ? {
                    top: `${(placed().sources.get(h.id)?.y ?? at().y) - at().y}px`,
                  } : undefined}>
                    <button
                      type="button"
                      class="cst-root cst-head open"
                      data-key={h.node.key}
                      aria-haspopup="dialog"
                    >
                      <HeadName item={h.node.item} />
                    </button>
                  </div>
                )}
              </For>
              <Show when={selective(p.branch) && remainingRows(p.hub, p.branch, rows()).length}>
                <PreviewBox hub={p.hub} branch={p.branch} floating />
              </Show>
              <For each={roots.filter((g) => !canOpen(g, target))}>
                {(g) => <GroupView hub={p.hub} branch={p.branch} group={g} />}
              </For>
            </div>
          }
        >
          <Show when={selective(p.branch)} fallback={<button
            type="button"
            class="cst-face"
            aria-expanded="false"
            title={
              openable.length === 1
                ? `unfold what comes from ${term(openable[0].head.item)}`
                : `show its ${openable.length} roots`
            }
            onClick={act}
          >
            <span class="cst-fam">{p.branch.name} Descendants</span>
            <span class="cst-words cst-preview">
              <For each={lines()}>
                {(words) => <span class="cst-preview-line"><WordList words={words} live={false} /></span>}
              </For>
            </span>
          </button>}>
            <PreviewBox hub={p.hub} branch={p.branch} />
          </Show>
        </Show>
      </section>
    );
  };

  // The root: the one disc, in the middle.
  const HubView = (p: { hub: Hub }) => {
    const item = p.hub.node.item;
    const r = ROOT_R;
    const at = () => placed().hubs.get(p.hub.id);
    const shownTerm = term(item);
    const meaning = item.gloss?.[0] ? brief(item.gloss[0]) : "";
    // The word and its meaning are set at one size: as large as lets the
    // word fit on one line and the meaning on two.
    const size = Math.max(15, 36 * Math.min(1, 7 / [...shownTerm].length, meaning ? 13 / [...meaning].length : 1));
    return (
      <Show when={at()}>
        {(c) => (
          <button
            type="button"
            class="cst-hub"
            classList={{ root: true, current: current() === p.hub }}
            style={{
              "--c": "255,255,255",
              left: `${c().x - r}px`,
              top: `${c().y - r}px`,
              width: `${r * 2}px`,
              height: `${r * 2}px`,
            }}
            title="Reset constellation"
            aria-label="Reset constellation"
            onClick={(e) => {
              e.stopPropagation();
              startOver();
            }}
          >
            <span class="cst-hub-term" style={{ "font-size": `${size}px` }}>
              {shownTerm}
            </span>
            <Show when={meaning}>
              <span class="cst-hub-gloss" style={{ "font-size": `${size}px` }}>
                {meaning}
              </span>
            </Show>
          </button>
        )}
      </Show>
    );
  };

  // A hub opened before its place is known is centred once it has one.
  createEffect(
    on(
      () => placed().hubs.get(current().id),
      (p, before) => {
        if (p && !before) centreOn(p);
      },
      { defer: true }
    )
  );

  return (
    <div
      ref={stageEl}
      class="cst-stage"
      classList={{ dragging: dragging() }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onFocusIn={onFocusIn}
      onClick={onClick}
      onPointerOver={onPointerOver}
      onPointerOut={onPointerOut}
    >
      <div
        class="cst-pan"
        classList={{ gliding: gliding() }}
        style={{ left: `${view().x}px`, top: `${view().y}px` }}
      >
        <div class="cst-world" style={{ zoom: view().k }}>
          <svg class="cst-links" aria-hidden="true">
            <For each={links()}>
              {(l) => {
                const [hue, d, x, y, x0, y0] = l.split("|");
                return (
                  <g style={{ "--c": hue }}>
                    <path class="glow" d={d} />
                    <path d={d} />
                    <circle cx={x} cy={y} r="4" />
                    <Show when={x0}>
                      <circle cx={x0} cy={y0} r="3" />
                    </Show>
                  </g>
                );
              }}
            </For>
          </svg>
          <For each={[...languageLabels().values()]}>
            {(label) => <span class="cst-language-label" style={{ left: `${label.x}px`, top: `${label.y}px`, "--c": label.hue }}>{label.name}</span>}
          </For>
          <For each={hubs()}>
            {(h) => (
              <>
                {/* Only the root is a disc: an opened head is a row in its
                    blob. */}
                <Show when={!h.parent}>
                  <HubView hub={h} />
                </Show>
                <For each={h.branches}>{(b) => <BlobView hub={h} branch={b} />}</For>
              </>
            )}
          </For>
        </div>
      </div>

      <Show when={rootHub.branches.length === 0}>
        <p class="cst-none">
          No {target.name} words are recorded as coming from {term(root.item)}.
        </p>
      </Show>
      <nav class="cst-ui cst-trail" aria-label="opened words">
        <For each={trail()}>
          {(h, i) => (
            <>
              <Show when={i() > 0}>
                <span aria-hidden="true">›</span>
              </Show>
              <button
                type="button"
                classList={{ current: current() === h }}
                style={{ "--c": h.hue ?? "255,255,255" }}
                onClick={() => focus(h)}
              >
                {term(h.node.item)}
              </button>
            </>
          )}
        </For>
      </nav>
      <p class="cst-ui cst-summary">
        {rootHub.branches.length} {rootHub.branches.length === 1 ? "branch" : "branches"} ·{" "}
        {rootWords.toLocaleString()} {target.name} {rootWords === 1 ? "word" : "words"}
      </p>
      <Show when={starts().length > 1}>
        <nav class="cst-ui cst-starts" aria-label="start from">
          <span>start from</span>
          <For each={starts()}>
            {(it) => (
              <button
                type="button"
                classList={{ current: it.id === root.item.id }}
                aria-current={it.id === root.item.id ? "true" : undefined}
                title={it.lang.name}
                onClick={() => startFrom(it)}
              >
                {term(it)}
              </button>
            )}
          </For>
        </nav>
      </Show>
      <p class="cst-ui cst-hint">Time runs outward; spacing is approximate. Click to unfold. Drag or scroll to move.</p>
      <div class="cst-ui cst-controls" role="group" aria-label="canvas">
        <Show when={hubs().length > 1}>
          <button type="button" class="wide" onClick={startOver}>
            start over
          </button>
        </Show>
        <button
          type="button"
          aria-label="zoom out"
          onClick={() => zoomAt(stage().w / 2, stage().h / 2, view().k / 1.25, true)}
        >
          −
        </button>
        <button
          type="button"
          aria-label="zoom in"
          onClick={() => zoomAt(stage().w / 2, stage().h / 2, view().k * 1.25, true)}
        >
          +
        </button>
      </div>
    </div>
  );
}
