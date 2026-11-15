import "./constellation.css";
import { Forest, TreeNode } from "../ety/model";
import { close as closePopover, hoverEnter, hoverLeave, pin, popover } from "../ety/popover";
import { useGoTo } from "../search/navigate";
import { Item, Lang, TreeKind, term } from "../search/types";
import { cognates, constellationStart, lastRequest, showGlosses } from "../state";
import { Branch, Group, LangTree, branchesOf, canOpen } from "./branches";
import { Body, HubSpec, Layout, Link, Point, Rect, Size, bounds, join, layout } from "./layout";

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

// A word at the centre of its branches: the tree's root, or a head the
// reader opened.
interface Hub {
  id: string;
  node: TreeNode;
  parent: Hub | null;
  // The blob it was opened from. A head is drawn once: when the blob held
  // nothing else, the hub's disc stands in the blob's place (`replaces`);
  // otherwise the blob stays, the head's name in it stands for the hub, and
  // the hub has no disc.
  via: string | null;
  replaces: boolean;
  branches: Branch[];
  // The colour of the branch it was opened from, which its own branches keep.
  hue: string | null;
  hues: Map<string, string>;
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
const ROOT_R = 88;
const HUB_R = 76;
// How many are listed before "+n more" (a couple over is shown in full): a
// branch's words in one list, its roots once broken out, a root's words.
const FLAT = 10;
const GROUPS = 6;
const WORDS = 6;
const MIN_ZOOM = 0.35;
const MAX_ZOOM = 1.6;
// The first view keeps clear of the stage's edges and its corner text, and
// is laid out to fit at a zoom the words still read at.
const PAD_X = 48;
const PAD_Y = 96;
const READABLE = 0.85;

const blobId = (hub: Hub, branch: Branch) => `${hub.id}/${branch.key}`;
const radius = (hub: Hub) => (hub.parent ? HUB_R : ROOT_R);

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
// the root in the middle, a blob for each branch of the language tree they
// came down, and in each blob the branch's own words with their descendants
// in the reader's language. Opening one of those words unfolds its branches
// in turn, beside what is already there.
export default function Constellation(props: { forest: Forest }) {
  const [langs] = createResource(() => import("virtual:lang-tree") as Promise<LangTree>);
  return (
    <Show
      when={props.forest.roots.length === 1}
      fallback={<p class="empty">The filters hide the word this constellation starts from.</p>}
    >
      <Show when={langs()} fallback={<p class="empty" role="status">Loading languages…</p>}>
        {(l) => <Stage forest={props.forest} langs={l()} />}
      </Show>
    </Show>
  );
}

function Stage(props: { forest: Forest; langs: LangTree }) {
  let stageEl!: HTMLDivElement;
  const root = props.forest.roots[0];
  const target: Lang = (props.forest.selected ?? root).item.lang;

  const makeHub = (
    node: TreeNode,
    parent: Hub | null,
    via: string | null,
    hue: string | null,
    replaces = false
  ): Hub => {
    const branches = branchesOf(node, target, props.langs);
    const keys = branches.map((b) => b.key);
    return {
      id: String(node.item.id),
      node,
      parent,
      via,
      replaces,
      branches,
      hue,
      hues: hue ? new Map(keys.map((k) => [k, hue])) : huesOf(keys),
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
  // Blobs broken out into their roots, and those showing every root.
  const [broken, setBroken] = createSignal(new Set<string>(), { equals: false });
  const [moreGroups, setMoreGroups] = createSignal(new Set<string>(), { equals: false });
  const [moreWords, setMoreWords] = createSignal(new Set<string>(), { equals: false });
  const toggle = (set: Set<string>, key: string) => (set.has(key) ? set.delete(key) : set.add(key), set);

  // Blobs are measured as drawn; the layout follows their real sizes.
  const observer = new ResizeObserver((entries) => {
    const map = sizes();
    let changed = false;
    for (const entry of entries) {
      const el = entry.target as HTMLElement;
      const id = el.dataset.blob!;
      const now = { w: el.offsetWidth, h: el.offsetHeight };
      const had = map.get(id);
      if (now.w && now.h && (!had || had.now.w !== now.w || had.now.h !== now.h)) {
        map.set(id, { now, seat: { w: now.w, h: Math.max(now.h, had?.seat.h ?? 0) } });
        changed = true;
      }
    }
    if (changed) setSizes(map);
  });
  onCleanup(() => observer.disconnect());

  // The room the first view has: the stage as first seen, at a zoom the
  // words still read at. Fixed from then on, so resizing the window does not
  // shuffle the blobs.
  const [frame, setFrame] = createSignal<Size>();
  const placed = createMemo<Layout>((before) => {
    const measured = sizes();
    const room = frame();
    if (!room) return before;
    return layout(
      hubs().map(
        (h): HubSpec => ({
          id: h.id,
          r: radius(h),
          from: h.parent && h.via ? { hub: h.parent.id, blob: h.via, within: !h.replaces } : null,
          blobs: h.branches.map((b) => ({ id: blobId(h, b), size: measured.get(blobId(h, b))?.seat })),
        })
      ),
      room,
      before
    );
  }, { hubs: new Map(), blobs: new Map() });
  // Where a blob is drawn: its seat, or when it has shrunk, the end of its
  // seat nearer its hub.
  const drawn = (hub: Hub, id: string): Rect | undefined => {
    const seat = placed().blobs.get(id);
    const now = sizes().get(id)?.now;
    const c = placed().hubs.get(hub.id);
    if (!seat || !now || !c || now.h >= seat.h) return seat;
    const below = c.y > seat.y + seat.h / 2;
    return { x: seat.x, y: below ? seat.y + seat.h - now.h : seat.y, w: seat.w, h: now.h };
  };

  // Blobs whose one head has been opened: the hub stands in their place.
  const replaced = createMemo(() => new Set(hubs().filter((h) => h.replaces).map((h) => h.via)));

  // One string per line, so a line that has not moved keeps its element.
  const links = createMemo(() => {
    const at = placed();
    const gone = replaced();
    const out: string[] = [];
    const push = (hue: string, l: Link) => out.push([hue, l.d, l.end.x.toFixed(1), l.end.y.toFixed(1)].join("|"));
    // What stands for a hub: its disc, or the blob its head is named in.
    const body = (h: Hub): Body | undefined => {
      if (h.parent && !h.replaces) return h.via ? drawn(h.parent, h.via) : undefined;
      const c = at.hubs.get(h.id);
      return c && { c, r: radius(h) };
    };
    for (const h of hubs()) {
      const me = body(h);
      if (!me) continue;
      // A disc in a blob's place is joined to the hub before it, as the
      // blob was.
      const before = h.parent && h.replaces ? body(h.parent) : undefined;
      if (before && h.hue) push(h.hue, join(before, me));
      for (const b of h.branches) {
        const id = blobId(h, b);
        const box = gone.has(id) ? undefined : drawn(h, id);
        if (box) push(h.hues.get(b.key)!, join(me, box));
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

  // Centre on a hub with its blobs: what stands for it (its disc, or the
  // blob its head is named in) and everything that branches off.
  const centreOnHub = (hub: Hub) => {
    const c = placed().hubs.get(hub.id);
    const r = radius(hub);
    const rects: Rect[] = [];
    if (hub.parent && !hub.replaces && hub.via) {
      const via = drawn(hub.parent, hub.via);
      if (via) rects.push(via);
    } else if (c) rects.push({ x: c.x - r, y: c.y - r, w: r * 2, h: r * 2 });
    for (const b of hub.branches) {
      const box = replaced().has(blobId(hub, b)) ? undefined : drawn(hub, blobId(hub, b));
      if (box) rects.push(box);
    }
    if (!rects.length) return;
    const x0 = Math.min(...rects.map((b) => b.x));
    const x1 = Math.max(...rects.map((b) => b.x + b.w));
    const y0 = Math.min(...rects.map((b) => b.y));
    const y1 = Math.max(...rects.map((b) => b.y + b.h));
    centreOn({ x: (x0 + x1) / 2, y: (y0 + y1) / 2 });
  };
  // A hub just opened has its blobs measured and placed a frame or two on.
  const focus = (hub: Hub) => {
    setCurrent(hub);
    centreOnHub(hub);
    requestAnimationFrame(() => requestAnimationFrame(() => current() === hub && centreOnHub(hub)));
  };
  const open = (hub: Hub, branch: Branch, g: Group) => {
    const id = String(g.head.item.id);
    let next = hubs().find((h) => h.id === id);
    if (!next) {
      next = makeHub(g.head, hub, blobId(hub, branch), hub.hues.get(branch.key)!, branch.groups.length === 1);
      setHubs([...hubs(), next]);
    }
    closePopover();
    focus(next);
  };
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
    const v = rootView();
    if (v) glide(v);
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
  const wordFromEvent = (e: Event): [TreeNode, HTMLElement] | null => {
    const el = (e.target as Element).closest<HTMLElement>(".cst-word");
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
    const el = (e.target as Element).closest(".cst-word");
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

  const GroupView = (p: { hub: Hub; branch: Branch; group: Group }) => {
    const key = `${blobId(p.hub, p.branch)}/${p.group.head.item.id}`;
    const item = p.group.head.item;
    const all = () => moreWords().has(key);
    const words = () =>
      all() || p.group.words.length <= WORDS + 2 ? p.group.words : p.group.words.slice(0, WORDS);
    // (A hub's own words are filed under the hub itself, which is no head.)
    const opened = () => !p.group.own && hubs().some((h) => h.id === String(item.id));
    const head = (
      <>
        <Show when={item.lang.name !== p.branch.name}>
          <span class="cst-head-lang">{item.lang.name}</span>
        </Show>
        <span class="cst-head-term">{term(item)}</span>
        <Show when={item.romanization}>
          <span class="cst-rom">{item.romanization}</span>
        </Show>
        <Show when={showGlosses() && item.gloss?.[0]}>
          <span class="cst-gloss">{item.gloss![0]}</span>
        </Show>
      </>
    );
    // A root that can be opened is one thing to click, words and all. Once
    // open it keeps only its name here, which stands for its hub: its words
    // are listed in the blobs that branch off, by branch.
    if (canOpen(p.group, target))
      return (
        <button
          type="button"
          class="cst-group"
          aria-expanded={opened()}
          title={opened() ? `go to ${term(item)}` : `unfold what comes from ${term(item)}`}
          onClick={() => open(p.hub, p.branch, p.group)}
        >
          <span class="cst-head" classList={{ open: opened(), current: current().id === String(item.id) }}>
            {head}
          </span>
          <Show when={!opened()}>
            <span class="cst-words">
              <WordList words={words()} live={false} />
              <Show when={words().length < p.group.words.length}>
                <span class="cst-note">+{p.group.words.length - words().length} more</span>
              </Show>
            </span>
          </Show>
        </button>
      );
    return (
      <div class="cst-group">
        <Show
          when={!p.group.own}
          fallback={
            <Show when={p.hub.node.item.lang.id !== target.id}>
              <span class="cst-head static">straight from {term(p.hub.node.item)}</span>
            </Show>
          }
        >
          <span class="cst-head static">{head}</span>
        </Show>
        <p class="cst-words">
          <WordList words={words()} live />
          <Show when={all() || words().length < p.group.words.length}>
            <button
              type="button"
              class="cst-more"
              aria-expanded={all()}
              onClick={() => setMoreWords(toggle(moreWords(), key))}
            >
              {all() ? "fewer" : `+${p.group.words.length - words().length} more`}
            </button>
          </Show>
        </p>
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
    // A branch starts as one list of its words, and the whole blob is one
    // thing to click. With a single root that opens the root; with several
    // the blob breaks out into them, each then opened by a click of its own.
    const roots = p.branch.groups;
    const only = roots.length === 1 ? roots[0] : null;
    const anyOpen = () => roots.some((g) => !g.own && hubs().some((h) => h.id === String(g.head.item.id)));
    const brokenOut = () => !only && (broken().has(id) || anyOpen());
    const face = () => (only ? canOpen(only, target) : !brokenOut());
    const flat = () => (p.branch.words.length <= FLAT + 2 ? p.branch.words : p.branch.words.slice(0, FLAT));
    const all = () => moreGroups().has(id);
    const groups = () => (all() || roots.length <= GROUPS + 2 ? roots : roots.slice(0, GROUPS));
    // Breaking out makes the blob taller, so it may move: centre on it where
    // it comes to rest.
    const centre = () =>
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          const r = rect();
          if (r) centreOn({ x: r.x + r.w / 2, y: r.y + r.h / 2 });
        })
      );
    const act = () => {
      if (only) return open(p.hub, p.branch, only);
      closePopover();
      setBroken(toggle(broken(), id));
      setCurrent(p.hub);
      centre();
    };
    const Name = () => (
      <>
        <span>{p.branch.name}</span>
        <span class="cst-count">
          {p.branch.wordCount.toLocaleString()} {p.branch.wordCount === 1 ? "word" : "words"}
          {only ? "" : ` · ${roots.length} roots`}
        </span>
      </>
    );
    return (
      <section
        ref={el}
        class="cst-blob"
        classList={{
          shown: shown() && !!rect(),
          // Lit: the current hub's blobs and the one it was opened from.
          dim: current() !== p.hub && current().via !== id,
          replaced: replaced().has(id),
          narrow: p.branch.wordCount <= 4,
        }}
        data-blob={id}
        inert={replaced().has(id)}
        style={{ "--c": p.hub.hues.get(p.branch.key), left: `${at().x}px`, top: `${at().y}px` }}
        aria-label={`${p.branch.name}: ${p.branch.wordCount} ${target.name} words`}
      >
        <Show
          when={face()}
          fallback={
            <div class="cst-body">
              <Show
                when={!only}
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
                  disabled={anyOpen()}
                  title={anyOpen() ? undefined : "fold back into one list"}
                  onClick={act}
                >
                  <Name />
                </button>
              </Show>
              <For each={groups()}>{(g) => <GroupView hub={p.hub} branch={p.branch} group={g} />}</For>
              <Show when={all() || groups().length < roots.length}>
                <button
                  type="button"
                  class="cst-more cst-more-groups"
                  aria-expanded={all()}
                  onClick={() => setMoreGroups(toggle(moreGroups(), id))}
                >
                  {all() ? "fewer roots" : `+${roots.length - groups().length} more roots`}
                </button>
              </Show>
            </div>
          }
        >
          <button
            type="button"
            class="cst-face"
            aria-expanded={only ? undefined : false}
            title={only ? `unfold what comes from ${term(only.head.item)}` : `show its ${roots.length} roots`}
            onClick={act}
          >
            <span class="cst-fam">
              <Name />
            </span>
            <span class="cst-words">
              <WordList words={flat()} live={false} />
              <Show when={flat().length < p.branch.words.length}>
                <span class="cst-note">+{p.branch.words.length - flat().length} more</span>
              </Show>
            </span>
          </button>
        </Show>
      </section>
    );
  };

  const HubView = (p: { hub: Hub }) => {
    const item = p.hub.node.item;
    const r = radius(p.hub);
    const at = () => placed().hubs.get(p.hub.id);
    const shownTerm = term(item);
    const count = new Set(p.hub.branches.flatMap((b) => b.groups.flatMap((g) => g.words.map((w) => w.item.term)))).size;
    return (
      <Show when={at()}>
        {(c) => (
          <button
            type="button"
            class="cst-hub"
            classList={{ root: !p.hub.parent, current: current() === p.hub }}
            style={{
              "--c": p.hub.hue ?? "255,255,255",
              left: `${c().x - r}px`,
              top: `${c().y - r}px`,
              width: `${r * 2}px`,
              height: `${r * 2}px`,
            }}
            title={current() === p.hub ? "word details" : "centre on this word"}
            onClick={(e) => (current() === p.hub ? pin(p.hub.node, e.currentTarget) : focus(p.hub))}
          >
            <span class="cst-hub-lang">{item.lang.name}</span>
            <span
              class="cst-hub-term"
              style={{ "font-size": `${Math.max(15, Math.min(1, 7 / [...shownTerm].length) * (p.hub.parent ? 30 : 36))}px` }}
            >
              {shownTerm}
            </span>
            <Show when={item.gloss?.[0]}>
              <span class="cst-hub-gloss">{item.gloss![0]}</span>
            </Show>
            <Show when={p.hub.parent}>
              <span class="cst-hub-count">
                {count.toLocaleString()} {count === 1 ? "word" : "words"}
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
                const [hue, d, x, y] = l.split("|");
                return (
                  <g style={{ "--c": hue }}>
                    <path class="glow" d={d} />
                    <path d={d} />
                    <circle cx={x} cy={y} r="4" />
                  </g>
                );
              }}
            </For>
          </svg>
          <For each={hubs()}>
            {(h) => (
              <>
                {/* A head opened in a blob that stays has no disc: its name
                    there stands for it. */}
                <Show when={!h.parent || h.replaces}>
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
      <p class="cst-ui cst-hint">Click a branch to unfold it. Drag or scroll to move.</p>
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
