// The constellation view: signals, measurement and rendering. What the
// words here mean (hub, blob, rack, lane, head, card, tray, stem) and which
// module decides what is in client/README.md, under "Constellation".
import "./constellation.css";
import { Forest, TreeNode } from "../ety/model";
import { close as closePopover, hoverEnter, hoverLeave, pin, popover } from "../ety/popover";
import { useGoTo } from "../search/navigate";
import { Item, Lang, TreeKind, term } from "../search/types";
import { cognates, constellationStart, lastRequest, setShowRare, showGlosses, showRare } from "../state";
import { Branch, Common, ENGLISH, Group, LangTree, isProto, isUndefinedRoot, canOpen, previewLines, previewRows, restrictBranches, type PreviewRow, type Reading } from "./branches";
import { selectWords } from "./selection";
import { blobId, expand, hueOf, makeHub, type Hub } from "./hubs";
import { cables, type Cable } from "./cables";
import { createCanvas } from "./canvas";
import { BlobSpec, Layout, Point, Rect, Size, SOURCE_HEIGHT, LANGUAGE_HEADING_SPACE, CARD_WIDTH, NARROW_CARD_WIDTH, center, enclosing, bounds, breathe, layout, rootDirection } from "./layout";
import { chronological, groupLanguageLanes, languageTimes } from "./chronology";
import type { LayoutPeriod } from "./chronology";
import type { Frequencies } from "./representatives";
import { newestFirst } from "./expansion";
import { compactBranches, compactVisibleLanes } from "./density";
import type { DensityNode } from "./density";
import { languageWidths, type LaneContent } from "./sizing";
import { dominantLanguage, gatherLanguage } from "./magnetism";
import { languageAnchors } from "./labels";
import type { LanguageAnchor } from "./labels";
import { expansionView, rootOverview } from "./camera";

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
  // Only English has a word list, so only it has a rare-words switch.
  const english = target.code === ENGLISH;
  const times = languageTimes(props.periods, props.langs, target.code, new Date().getFullYear());
  const { common, scores, branchesFor } = selectWords(root, target, props.langs, props.common, props.frequencies, times);
  const rootHub = makeHub(branchesFor, root, null, null, null);
  const rootWords = new Set(rootHub.branches.flatMap((b) => b.groups.flatMap((g) => g.words.map((w) => w.item.term)))).size;

  const [hubs, setHubs] = createSignal<Hub[]>([rootHub]);
  const [current, setCurrent] = createSignal<Hub>(rootHub);
  const hubBranches = createMemo(() => new Map(hubs().map((h) => [h, h.branches])));
  // The heads opened in each blob, in the order they were opened.
  const openedByBlob = createMemo(() => {
    const byBlob = new Map<string, Hub[]>();
    for (const h of hubs()) if (h.via) byBlob.set(h.via, [...(byBlob.get(h.via) ?? []), h]);
    return byBlob;
  });
  const openedIn = (blob: string): Hub[] => openedByBlob().get(blob) ?? [];
  let revealSequence = 0;
  const [recent, setRecent] = createSignal(new Set<string>());
  let recentTimer: number | undefined;
  const clearRecent = () => {
    window.clearTimeout(recentTimer);
    setRecent(new Set<string>());
  };
  onCleanup(() => window.clearTimeout(recentTimer));
  // A blob's size as drawn, and the room it holds on the canvas: the tallest
  // it has been. A blob that shrinks (a head opened, a list folded) keeps its
  // seat, so nothing round it moves.
  const [sizes, setSizes] = createSignal(new Map<string, { now: Size; seat: Size }>(), { equals: false });
  // In a rack, how far down its blob each head's row is centred: its line
  // leaves the blob there.
  const [rowMids, setRowMids] = createSignal(new Map<string, number>(), { equals: false });
  const [previewHeights, setPreviewHeights] = createSignal(new Map<string, number>(), { equals: false });
  const [previewWidths, setPreviewWidths] = createSignal(new Map<string, number>(), { equals: false });

  const selective = (b: Branch) => b.groups.some((g) => canOpen(g, target));
  const reading: Reading = { target, children: branchesFor, common, scores };
  const previewCache = new WeakMap<Branch, PreviewRow[]>();
  const trayCache = new WeakMap<Branch, PreviewRow[]>();
  const previewsFor = (b: Branch, splitSources: boolean) => {
    const cache = splitSources ? trayCache : previewCache;
    let rows = cache.get(b);
    if (!rows) {
      rows = previewRows(b, reading, { splitSources });
      cache.set(b, rows);
    }
    return rows;
  };
  const previewsOf = (hub: Hub, b: Branch) => previewsFor(b, descendantTray(hub, b));
  const singleWords = (b: Branch, words: TreeNode[]) => words.length > 1 && words.every((w) =>
    previewLines(b, reading, { combineSingles: false }).some((line) => line.length === 1 && line[0].item.term === w.item.term));
  const unopenedWords = (hub: Hub, b: Branch, words: TreeNode[], groups = b.groups) => words.filter((word) =>
    groups.some((g) => g.words.some((w) => w.item.term === word.item.term) && !openedIn(blobId(hub, b)).some((h) =>
      h.node.item.id === g.head.item.id && (!h.scope || h.scope.words.has(word.item.term)))));
  const rowKey = (hub: Hub, b: Branch, index: number) => {
    const row = previewsOf(hub, b)[index];
    return `${blobId(hub, b)}:row:${row.groups.map((g) => g.head.item.id).join(",")}:${row.words.map((w) => w.item.term).join(",")}`;
  };
  const previewBoxId = (hub: Hub, b: Branch) => `${blobId(hub, b)}:preview`;
  const remainingRows = (hub: Hub, b: Branch, heads: Hub[]) => previewsOf(hub, b).map((row, i) => ({ row, i }))
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
        const widths = previewWidths();
        if (widths.get(el.dataset.previewBox) !== el.offsetWidth) {
          widths.set(el.dataset.previewBox, el.offsetWidth);
          setPreviewWidths(widths);
        }
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
        map.set(id, { now, seat: { w: now.w, h: clusteredBlobs().has(id)
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
  const [availableHeight, setAvailableHeight] = createSignal(700);
  const [laneContent, setLaneContent] = createSignal<LaneContent[]>([]);
  const laneWidths = createMemo(() => languageWidths(laneContent(), availableWidth() * READABLE));
  // Measure content outside its assigned grid constraints. Do not feed an
  // observed, stretched card width back into the lane's preferred width.
  const intrinsicWidth = (el: HTMLElement) => {
    const copy = el.cloneNode(true) as HTMLElement;
    Object.assign(copy.style, { position: "absolute", visibility: "hidden", width: "max-content",
      minWidth: "0", maxWidth: "none", whiteSpace: "nowrap", pointerEvents: "none" });
    el.parentElement!.append(copy);
    // offsetWidth rounds to the nearest pixel and can make a fitting word wrap.
    const width = Math.ceil(parseFloat(getComputedStyle(copy).width));
    copy.remove();
    return width;
  };
  const measureContent = () => {
    if (!stageEl) return;
    const content: LaneContent[] = [];
    for (const el of stageEl.querySelectorAll<HTMLElement>(".cst-blob")) {
      const tray = el.querySelector<HTMLElement>(".cst-descendant-tray");
      const roots = [...el.querySelectorAll<HTMLElement>(".cst-head")];
      if (!tray && !roots.length) continue; // Initial overview clusters keep their simple shape.
      content.push({ language: el.dataset.language!, root: Math.max(0, ...roots.map(intrinsicWidth)),
        ...(tray ? {
          source: Math.max(0, ...[...tray.querySelectorAll<HTMLElement>(".cst-tray-source")].map(intrinsicWidth)),
          words: Math.max(0, ...[...tray.querySelectorAll<HTMLElement>(".cst-words")].map(intrinsicWidth)),
        } : {}) });
    }
    setLaneContent(content);
  };
  createEffect(() => {
    hubs(); showGlosses();
    const pending = requestAnimationFrame(measureContent);
    onCleanup(() => cancelAnimationFrame(pending));
  });
  onMount(() => { void document.fonts.ready.then(measureContent); });
  const [layoutVersion, setLayoutVersion] = createSignal(0);
  let placedVersion = 0;
  const clusteredHeads = (heads: Hub[]) => heads.length > 1 && heads.every((h) => isProto(h.node.item.lang.code));
  const clusteredBranch = (b: Branch, heads: Hub[]) => clusteredHeads(heads) || (selective(b) && heads.length > 0);
  // The blobs drawn as a column of separate source cards, whose height the
  // layout sets rather than their contents.
  const clusteredBlobs = createMemo(() => new Set(hubs().flatMap((h) => h.branches
    .filter((b) => clusteredBranch(b, openedIn(blobId(h, b)))).map((b) => blobId(h, b)))));
  // A family's overview remains a plain cluster. Once it has individual
  // roots, its remaining and downstream previews become descendant trays.
  const descendantTray = (h: Hub, b: Branch) => {
    let family = blobId(h, b);
    for (let ancestor: Hub | null = h; ancestor?.via; ancestor = ancestor.parent) family = ancestor.via;
    return hubs().some((child) => child.via === family && !isUndefinedRoot(child.node));
  };
  const entriesFor = (h: Hub, b: Branch, heads: Hub[]): { id: string; head: Hub | undefined }[] => selective(b) && heads.length > 0
    ? [
        ...(remainingRows(h, b, heads).length ? [{ id: previewBoxId(h, b), head: undefined }] : []),
        ...newestFirst(heads).map((head) => ({ id: head.id, head })),
      ]
    : heads.map((head) => ({ id: head.id, head }));
  // A crowded family compresses all of its individual source cards,
  // including short side branches, while preserving the preview boxes.
  const chainIds = createMemo(() => {
    const densityNode = (h: Hub, b: Branch): DensityNode => {
      const id = blobId(h, b);
      const open = openedIn(id);
      const source = b.groups.length === 1 && open.length === 1 && !isUndefinedRoot(open[0].node)
        && remainingRows(h, b, open).length === 0;
      const endpoint = b.groups.every((g) => g.own) && b.words.length === 1;
      return { id, width: open.length || b.words.length > 4 ? CARD_WIDTH : NARROW_CARD_WIDTH,
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
    const blobsOf = (h: Hub, compactLanes = false, outwardY: 1 | -1 = 1): BlobSpec[] =>
      h.branches.map((b) => {
        const id = blobId(h, b);
        const heads = openedIn(id);
        const clustered = clusteredBranch(b, heads);
        const terminal = h.single || b.groups.every((g) => g.own);
        const stage = terminal ? target.code : b.key;
        const direction = h === rootHub
          ? Math.sin(rootDirection(h.branches.indexOf(b), h.branches.length)) < 0 ? -1 : 1
          : outwardY;
        return {
          id,
          terminal,
          stage,
          outwardY: direction,
          year: times.get(stage),
          clustered,
          compactChain: chainIds().has(id),
          sharedParents: all.filter((source) => source.shared.some((s) => blobId(s.host, s.branch) === id)).map((source) => source.id),
          // A rack is laid out at the size it is now, so its rows' lines and
          // the words level with them leave it where the rows are drawn; a
          // list keeps the room it has had. Proto clusters use current child
          // sizes so removed internal headings leave no empty space behind.
          size: ((m) => !m ? undefined
            : clustered ? { w: m.now.w, h: SOURCE_HEIGHT }
            : heads.length || compactLanes ? m.now : m.seat)(measured.get(id)),
          lanes: heads.length
            ? entriesFor(h, b, heads).map((entry) => entry.head
              ? { hub: entry.id, mid: mids.get(entry.id), revealed: entry.head.revealed,
                  blobs: blobsOf(entry.head, compactLanes || clustered, direction), inline: entry.head.single }
              : { hub: entry.id, mid: undefined, preview: true, height: previewHeights().get(entry.id) ?? 64, blobs: [] })
            : undefined,
        };
      });
    return { id: rootHub.id, r: ROOT_R, blobs: blobsOf(rootHub).map(groupLanguageLanes).map(chronological)
      .map((b) => breathe(b, availableHeight())) };
  });
  const baseLayout = createMemo<Layout>(
    (before) => {
      const room = frame();
      const version = layoutVersion();
      const previous = version === placedVersion ? before : undefined;
      placedVersion = version;
      return room ? layout(spec(), room, previous) : before;
    },
    { hubs: new Map(), blobs: new Map(), seats: new Map(), sides: new Map(), sources: new Map() }
  );
  // Until the reader moves the camera, the first view keeps fitting itself
  // as measurements settle.
  const [initialOverview, setInitialOverview] = createSignal(true);
  const canvas = createCanvas({
    stage: () => stageEl,
    minZoom: MIN_ZOOM,
    maxZoom: MAX_ZOOM,
    onTakeOver: () => setInitialOverview(false),
    closeDetails: (pinned) => {
      if (pinned || !popover()?.pinned) closePopover();
    },
    // The room the first view has: the stage as first seen, at a zoom the
    // words still read at. Fixed from then on, so resizing the window does
    // not shuffle the blobs.
    onResize: (now, first) => {
      const room = { w: (now.w * now.scale - PAD_X) / READABLE, h: (now.h * now.scale - PAD_Y) / READABLE };
      setAvailableWidth(room.w);
      setAvailableHeight(room.h);
      if (first) setFrame(room);
    },
  });
  const { view, setView, screen, gliding, dragging, glide, centreOn } = canvas;
  const placed = createMemo(() => {
    const v = view(), s = screen();
    if (initialOverview() || !s.w) return baseLayout();
    const viewport = { x: (s.x - v.x) / v.k, y: (s.y - v.y) / v.k, w: s.w / v.k, h: s.h / v.k };
    // Rank against the stable base, not our own reordered output: grouping
    // cannot change its own priority and start an oscillating layout loop.
    const base = baseLayout();
    const language = dominantLanguage(spec(), base, viewport);
    const grouped = language ? gatherLanguage(spec(), language) : spec();
    const arranged = language ? layout(grouped, frame(), base) : base;
    return compactVisibleLanes(arranged, grouped, viewport);
  });
  // Where a blob is drawn: its seat, or when it has shrunk, the end of its
  // seat nearer its hub.
  const drawn = (hub: Hub, id: string): Rect | undefined => {
    const seat = placed().blobs.get(id);
    const now = sizes().get(id)?.now;
    const c = placed().hubs.get(hub.id);
    if (openedIn(id).some((h) => placed().sources.has(h.id))) return seat;
    if (!seat || !now || !c || now.h >= seat.h) return seat;
    const below = c.y > seat.y + seat.h / 2;
    return { x: seat.x, y: below ? seat.y + seat.h - now.h : seat.y, w: seat.w, h: now.h };
  };

  // Root cards and descendant previews share the same language heading.
  // Gather actual visible faces rather than the virtual rack rectangles.
  const languageGroups = createMemo(() => {
    const groups = new Map<string, { name: string; hue: string; anchors: LanguageAnchor[] }>();
    for (const h of hubs()) for (const b of h.branches) {
      const id = blobId(h, b);
      const open = openedIn(id);
      const box = drawn(h, id);
      if (!box) continue;
      const key = b.key;
      let group = groups.get(key);
      if (!group) groups.set(key, (group = { name: b.name, hue: hueOf(h, b), anchors: [] }));
      if (clusteredBranch(b, open)) {
        for (const entry of entriesFor(h, b, open)) {
          const source = placed().sources.get(entry.id);
          if (source) group.anchors.push({ box: source, id: entry.id, preview: !entry.head });
        }
      } else group.anchors.push({ box, id, preview: !open.length && b.groups.some((g) => canOpen(g, target)) });
    }
    return groups;
  });

  const visibleLanguageAnchors = createMemo(() => {
    const v = view();
    const s = screen();
    const viewport = { x: (s.x - v.x) / v.k, y: (s.y - v.y) / v.k, w: s.w / v.k, h: s.h / v.k };
    return [...languageGroups()].flatMap(([language, g]) => languageAnchors(g.anchors, viewport)
      .map((anchor) => ({ ...anchor, language, name: g.name, hue: g.hue })));
  });
  // One heading per language code, where the model puts it before the
  // painted text is measured.
  const languageLabels = createMemo(() => visibleLanguageAnchors()
    .map((a) => ({ language: a.language, name: a.name, hue: a.hue, x: a.box.x + a.box.w / 2, y: a.box.y - LANGUAGE_HEADING_SPACE })));
  // Where each heading actually goes once its painted size is known (see the
  // measuring effect below): a point, or null when no card of the language
  // has room for it. A heading not measured yet stays where the model put it.
  const [labelPlacements, setLabelPlacements] = createSignal(new Map<string, Point | null>());
  const labelElement = (language: string) =>
    stageEl.querySelector<HTMLElement>(`[data-language-label="${CSS.escape(language)}"]`);

  // A cable that has not moved keeps its object, so <For> keeps its element
  // (and a fresh cable's accent is not restarted by unrelated changes).
  let keptCables = new Map<string, Cable>();
  const links = createMemo(() => {
    const kept = new Map<string, Cable>();
    const list = cables({
      hubs: hubs(), at: placed(), rootRadius: ROOT_R, recent: recent(), drawn,
      entries: (h, b) => entriesFor(h, b, openedIn(blobId(h, b))),
      clustered: (h, b) => clusteredBranch(b, openedIn(blobId(h, b))),
      remainingGroups: (h, b) => remainingRows(h, b, openedIn(blobId(h, b))).flatMap((i) => previewsOf(h, b)[i].groups),
    }).map((cable) => {
      const key = `${cable.hue} ${cable.fresh} ${cable.d}`;
      const same = keptCables.get(key) ?? cable;
      kept.set(key, same);
      return same;
    });
    keptCables = kept;
    return list;
  });

  const centreOnAll = (rects: Rect[]) => {
    const all = enclosing(rects);
    if (all) centreOn(center(all));
  };
  const visibleRects = (h: Hub, b: Branch): Rect[] => {
    const id = blobId(h, b);
    const sources = entriesFor(h, b, openedIn(id)).flatMap((e) => placed().sources.get(e.id) ?? []);
    const box = drawn(h, id);
    return sources.length ? sources : box ? [box] : [];
  };
  // A blob with everything racked off it.
  const rackRects = (hub: Hub, id: string): Rect[] => {
    const box = drawn(hub, id);
    return [
      ...(box ? [box] : []),
      ...openedIn(id).flatMap((k) => k.branches.flatMap((b) => rackRects(k, blobId(k, b)))),
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
    setInitialOverview(false);
    setCurrent(hub);
    centreOnHub(hub);
  };
  // Open heads in a blob, all those given at once: the blob becomes a rack,
  // each head a row in it with its own blobs in a lane beside it. A blob
  // with one head is a rack of one row.
  const rack = (hub: Hub, branch: Branch, groups: Group[], scope = hub.scope) => {
    setInitialOverview(false);
    const id = blobId(hub, branch);
    const before = new Set(hubs().map((h) => h.id));
    const { all, touched } = expand(hubs(), hub, branch, groups, scope, {
      target, branchesFor, fromTray: descendantTray(hub, branch), revealed: ++revealSequence,
    });
    setHubs(all);
    window.clearTimeout(recentTimer);
    setRecent(new Set(all.filter((h) => !before.has(h.id)).map((h) => h.id)));
    recentTimer = window.setTimeout(() => setRecent(new Set<string>()), 4000);
    closePopover();
    setCurrent(hub);
    settled(() => {
      const opened = all.filter((h) => h.via === id && touched.has(h));
      const sources = opened.flatMap((h) => placed().sources.get(h.id) ?? []);
      // After the last preview row disappears, its revealed historical
      // source cards retain the clicked side of the transition.
      const clicked = placed().sources.get(previewBoxId(hub, branch)) ?? enclosing(sources) ?? drawn(hub, id);
      const seen = new Set<string>();
      const frontier = (owner: Hub, b: Branch): Rect[] => {
        const key = blobId(owner, b);
        if (seen.has(key)) return [];
        seen.add(key);
        return [
          ...visibleRects(owner, b),
          ...all.filter((h) => h.via === key).flatMap((h) => [
            ...h.branches.flatMap((next) => frontier(h, next)),
            ...h.shared.flatMap((shared) => frontier(shared.host, shared.branch)),
          ]),
        ];
      };
      const revealed = [
        ...sources,
        ...opened.flatMap((h) => [
          ...h.branches.flatMap((b) => frontier(h, b)),
          ...h.shared.flatMap((shared) => frontier(shared.host, shared.branch)),
        ]),
      ];
      // Include the language headings in the fit as well as every stage
      // exposed by automatic expansion, not just the immediate children.
      const next = enclosing(revealed.map((r) => ({ ...r, y: r.y - LANGUAGE_HEADING_SPACE, h: r.h + LANGUAGE_HEADING_SPACE })));
      seen.clear();
      const context = enclosing([...(clicked ? [clicked] : []), ...frontier(hub, branch)]) ?? clicked;
      const languages = new Set([branch.key, ...all.filter((h) => !before.has(h.id))
        .flatMap((h) => h.branches.map((b) => b.key))]);
      const labelBounds = enclosing(languageLabels().filter((label) => languages.has(label.language)).map((label) => {
        const r = labelElement(label.language)?.getBoundingClientRect();
        const w = r ? r.width / view().k : 0;
        return { x: label.x - w / 2, y: label.y, w, h: r ? r.height / view().k : 20 };
      }));
      if (context && next && screen().w) glide(expansionView(context, next, screen(), labelBounds));
    });
  };
  // Back to one list: everything opened from the blob goes.
  const unrack = (hub: Hub, id: string) => {
    clearRecent();
    const gone = new Set<Hub>();
    for (const h of hubs()) if (h.via === id || (h.parent && gone.has(h.parent))) gone.add(h);
    closePopover();
    setHubs(hubs().filter((h) => !gone.has(h)));
    setCurrent(hub);
    settled(() => centreOnAll(rackRects(hub, id)));
  };
  // Lit: the current hub's blobs, and those racked off them.
  const lit = (hub: Hub): boolean => hub === current() || (!!hub.parent && lit(hub.parent));
  // Compose the initial branches around the root, with room to shift the
  // root slightly when an uneven branch would otherwise force a small zoom.
  const rootView = () => {
    const b = bounds(placed(), rootHub.id, ROOT_R, rootHub.branches.map((br) => blobId(rootHub, br)));
    const c = placed().hubs.get(rootHub.id);
    const s = screen();
    if (!b || !c || !s.w) return null;
    return rootOverview(c, b, s);
  };
  // Keep the overview fitted while initial measurements and fonts settle.
  // Once the reader interacts, layout updates must not move their camera.
  createEffect(() => {
    if (!initialOverview() || !rootHub.branches.every((b) => placed().blobs.has(blobId(rootHub, b)))) return;
    const v = rootView();
    if (!v) return;
    setView(v);
  });
  const startOver = () => {
    setInitialOverview(false);
    clearRecent();
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
        {(gloss) => (
          <span class="cst-gloss" title={gloss()}>
            {brief(gloss())}
          </span>
        )}
      </Show>
    </>
  );

  // A head that cannot be opened (its words are all there is under it), with
  // its words, each a button for its details.
  const GroupView = (p: { group: Group }) => {
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
    const row = previewsOf(p.hub, p.branch)[p.index];
    const id = rowKey(p.hub, p.branch, p.index);
    const words = () => unopenedWords(p.hub, p.branch, row.words, row.groups);
    const lines = () => {
      const terms = new Set(words().map((w) => w.item.term));
      return row.lines.map((line) => line.filter((w) => terms.has(w.item.term))).filter((line) => line.length);
    };
    return (
      <Show when={!descendantTray(p.hub, p.branch) && singleWords(p.branch, row.words)} fallback={
      <button
        type="button"
        class="cst-preview-row"
        data-row-choice={id}
        aria-expanded="false"
        aria-label={`Expand ${words().map((w) => w.item.term).join(", ")}`}
        disabled={!row.groups.some((g) => canOpen(g, target))}
        onClick={() => {
          const selected = new Set(words().map((w) => w.item.term));
          const groups = restrictBranches([{ ...p.branch, groups: row.groups }], selected)[0]?.groups ?? [];
          rack(p.hub, p.branch, groups, { id, words: selected });
        }}
      >
        <Show when={descendantTray(p.hub, p.branch)}>
          <span class="cst-tray-source">
            <For each={[...new Map(row.groups.map((g) => [g.head.item.id, g.head.item])).values()]}>{(item) =>
              <span class="cst-tray-source-entry"><span class="cst-tray-term">{term(item)}</span>
                <Show when={showGlosses() && item.gloss?.[0]}>{(gloss) => <span> – {brief(gloss())}</span>}</Show>
              </span>
            }</For>
          </span>
        </Show>
        <span class="cst-words">
          <For each={lines()}>{(words) => <span class="cst-preview-line"><WordList words={words} live={false} /></span>}</For>
        </span>
      </button>
      }>
        <div class="cst-preview-row cst-single-words" data-row-choice={id}>
          <span class="cst-words"><For each={words()}>{(word, i) => <>
            <button type="button" class="cst-single-word" aria-label={`Expand ${term(word.item)}`}
              onClick={(e) => {
                e.stopPropagation();
                const chosen = new Set([word.item.term]);
                rack(p.hub, p.branch, restrictBranches([p.branch], chosen)[0]?.groups ?? [], { id: `${id}:${word.item.id}`, words: chosen });
              }}>{term(word.item)}</button>{i() < words().length - 1 ? ", " : ""}
          </>}</For></span>
        </div>
      </Show>
    );
  };

  const PreviewBox = (p: { hub: Hub; branch: Branch; floating?: boolean }) => {
    let el!: HTMLDivElement;
    const id = previewBoxId(p.hub, p.branch);
    const remaining = () => remainingRows(p.hub, p.branch, openedIn(blobId(p.hub, p.branch)));
    onMount(() => observer.observe(el));
    onCleanup(() => observer.unobserve(el));
    return (
      <div ref={el} class="cst-row-face" classList={{ "cst-preview-box": p.floating, "cst-descendant-tray": descendantTray(p.hub, p.branch) }} data-preview-box={id}
        style={p.floating ? { top: `${(placed().sources.get(id)?.y ?? 0) - (drawn(p.hub, blobId(p.hub, p.branch))?.y ?? 0)}px` } : undefined}>
        <For each={remaining().map((index) => previewsOf(p.hub, p.branch)[index])}>{(row) =>
          <PreviewRow hub={p.hub} branch={p.branch} index={previewsOf(p.hub, p.branch).indexOf(row)} />
        }</For>
      </div>
    );
  };

  const BlobView = (p: { hub: Hub; branch: Branch }) => {
    let el!: HTMLElement;
    const id = blobId(p.hub, p.branch);
    const rect = () => drawn(p.hub, id);
    const laneWidth = () => laneWidths().get(p.branch.key);
    // Unfolds from its hub: drawn there first, then moved to its place.
    const [shown, setShown] = createSignal(false);
    createEffect(() => {
      if (rect() && !shown()) requestAnimationFrame(() => setShown(true));
    });
    onMount(() => {
      observer.observe(el);
      onCleanup(() => {
        observer.unobserve(el);
        // Forget its size, so a blob drawn again under this ID is measured afresh.
        setSizes((map) => {
          map.delete(id);
          return map;
        });
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
    const rows = () => newestFirst(openedIn(id));
    const openable = roots.filter((g) => canOpen(g, target));
    const lines = () => previewLines(p.branch, reading);
    // One named word needs one row, even when more language stages follow.
    // Keeping those stages visible must not add a separate language header.
    const compact = () => roots.length === 1 && rows().length === 1 && !isUndefinedRoot(rows()[0].node);
    const clustered = () => clusteredBranch(p.branch, rows());
    const face = () => openable.length > 0 && rows().length === 0;
    const act = () => rack(p.hub, p.branch, openable);
    // A head's one word: set as the head's row is, and a button for its
    // details.
    if (p.hub.single || (roots.length === 1 && roots[0].own && p.branch.words.length === 1)) {
      const word = p.branch.words[0];
      return (
        <section
          ref={el}
          class="cst-blob single"
          classList={{ shown: shown() && !!rect(), dim: !lit(p.hub), "compact-chain": chainIds().has(id), "sized-lane": !!laneWidth(), fresh: recent().has(p.hub.id) }}
          data-blob={id}
          data-language={p.branch.key}
          aria-label={`${word.item.lang.name}: ${term(word.item)}`}
          style={{ "--c": hueOf(p.hub, p.branch), "--lane-width": `${laneWidth()?.width ?? CARD_WIDTH}px`, left: `${at().x}px`, top: `${at().y}px` }}
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
          fresh: !clustered() && recent().has(p.hub.id),
          "descendant-tray": face() && descendantTray(p.hub, p.branch),
          "sized-lane": !!laneWidth(),
          "tray-column": descendantTray(p.hub, p.branch) && remainingRows(p.hub, p.branch, rows()).length > 0,
        }}
        data-blob={id}
        data-language={p.branch.key}
        style={{ "--c": hueOf(p.hub, p.branch), left: `${at().x}px`, top: `${at().y}px`,
          "--lane-width": `${laneWidth()?.width ?? CARD_WIDTH}px`,
          "--tray-source-width": ((w) => w ? `${w.source}px` : undefined)(laneWidth()),
          "--tray-width": `${previewWidths().get(previewBoxId(p.hub, p.branch)) ?? CARD_WIDTH}px`,
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
              <For each={rows()}>
                {(h) => (
                  <div class="cst-row" classList={{ fresh: recent().has(h.id) }} data-row={h.id} style={clustered() ? {
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
                {(g) => <GroupView group={g} />}
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

  // Labels are all-or-nothing, including during animated camera moves.
  // Measure the painted text so long names and browser zoom use real bounds,
  // and record where each goes in `labelPlacements`, which the labels read.
  let labelFrame = 0;
  createEffect(() => {
    view(); screen(); languageLabels(); gliding();
    cancelAnimationFrame(labelFrame);
    let settling = 4;
    const check = () => {
      if (!stageEl) return;
      const stageBox = stageEl.getBoundingClientRect();
      const visual = window.visualViewport;
      const pad = 6 / (visual?.scale ?? 1);
      const left = Math.max(stageBox.left, visual?.offsetLeft ?? 0) + pad;
      const top = Math.max(stageBox.top, visual?.offsetTop ?? 0) + pad;
      const right = Math.min(stageBox.right, (visual?.offsetLeft ?? 0) + (visual?.width ?? innerWidth)) - pad;
      const bottom = Math.min(stageBox.bottom, (visual?.offsetTop ?? 0) + (visual?.height ?? innerHeight)) - pad;
      const world = stageEl.querySelector<HTMLElement>(".cst-world")!.getBoundingClientRect();
      const placements = new Map<string, Point | null>();
      for (const { language } of languageLabels()) {
        const label = labelElement(language);
        const group = languageGroups().get(language);
        if (!label || !group) continue;
        const r = label.getBoundingClientRect();
        const k = label.offsetWidth ? r.width / label.offsetWidth : view().k;
        const viewport = { x: (left - world.x) / k, y: (top - world.y) / k,
          w: (right - left) / k, h: (bottom - top) / k };
        const anchor = languageAnchors(group.anchors, viewport, { w: r.width / k, h: r.height / k }, 0)[0];
        placements.set(language, anchor
          ? { x: anchor.box.x + anchor.box.w / 2, y: anchor.box.y - LANGUAGE_HEADING_SPACE }
          : null);
      }
      const before = labelPlacements();
      const same = before.size === placements.size && [...placements].every(([language, p]) => {
        const had = before.get(language);
        return had === p || (!!had && !!p && had.x === p.x && had.y === p.y);
      });
      if (!same) setLabelPlacements(placements);
      if (gliding() || --settling > 0) labelFrame = requestAnimationFrame(check);
    };
    labelFrame = requestAnimationFrame(check);
  });
  onCleanup(() => cancelAnimationFrame(labelFrame));

  return (
    <div
      ref={stageEl}
      class="cst-stage"
      classList={{ dragging: dragging() }}
      style={{ "--cst-card-width": `${CARD_WIDTH}px`, "--cst-narrow-width": `${NARROW_CARD_WIDTH}px` }}
      onPointerDown={canvas.handlers.onPointerDown}
      onPointerMove={canvas.handlers.onPointerMove}
      onPointerUp={canvas.handlers.onPointerUp}
      onPointerCancel={canvas.handlers.onPointerCancel}
      onFocusIn={canvas.handlers.onFocusIn}
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
              {(cable) => (
                <g classList={{ fresh: cable.fresh }} style={{ "--c": cable.hue }}>
                  <path class="glow" d={cable.d} />
                  <path d={cable.d} />
                </g>
              )}
            </For>
          </svg>
          <For each={languageLabels()}>
            {(label) => {
              const placed = () => labelPlacements().get(label.language);
              return (
                <span
                  class="cst-language-label"
                  data-language-label={label.language}
                  style={{
                    left: `${placed()?.x ?? label.x}px`,
                    top: `${placed()?.y ?? label.y}px`,
                    visibility: placed() === null ? "hidden" : "visible",
                    "--c": label.hue,
                  }}
                >
                  {label.name}
                </span>
              );
            }}
          </For>
          <For each={hubs()}>
            {(h) => (
              <>
                {/* Only the root is a disc: an opened head is a row in its
                    blob. */}
                <Show when={!h.parent}>
                  <HubView hub={h} />
                </Show>
                <For each={hubBranches().get(h)}>{(b) => <BlobView hub={h} branch={b} />}</For>
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
        {/* Only English has a word list to leave rare words out by. */}
        <Show when={english}>
          <button
            type="button"
            class="wide"
            aria-pressed={showRare()}
            title="Include obscure English words and derivatives of a word already listed"
            onClick={() => setShowRare(!showRare())}
          >
            rare words
          </button>
        </Show>
        <Show when={hubs().length > 1}>
          <button type="button" class="wide" onClick={startOver}>
            start over
          </button>
        </Show>
        <button
          type="button"
          aria-label="zoom out"
          onClick={() => canvas.zoomBy(1 / 1.25)}
        >
          −
        </button>
        <button
          type="button"
          aria-label="zoom in"
          onClick={() => canvas.zoomBy(1.25)}
        >
          +
        </button>
      </div>
    </div>
  );
}
