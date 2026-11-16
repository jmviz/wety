// Where the constellation's root and blobs go.
//
// The root is a disc with its blobs round it. A blob whose heads have been
// opened is a rack: the heads are rows in the blob, which keeps its own
// size, and each row's blobs stand in a lane beside it, the lanes one under
// another in the rows' order, so the lines from heads to blobs fan out side
// by side. A head with one word under it has that word level with its row
// instead, at the end of a short straight line. A lane's blob may be a rack
// in turn, and so on outwards. Proto-language racks instead give each source
// a separate card: its vertical position follows the centre of its subtree.

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  w: number;
  h: number;
}

// Top-left corner and size.
export interface Rect extends Point, Size {}

export interface BlobSpec {
  id: string;
  // As measured; a blob not measured yet has no size and is left out.
  size: Size | undefined;
  // The heads opened in it, in the order of their rows.
  lanes?: LaneSpec[];
  // A final word or list in the reader's language. These share the outer
  // edge of their root branch, even when their paths have different lengths.
  terminal?: boolean;
  // Approximate language chronology for this branch's columns.
  stage?: string;
  year?: number;
  // Assigned by chronological(): distance from the parent box's edge.
  gap?: number;
  // An opened proto-language rack: each source gets its own card centred
  // on its subtree, rather than staying inside a shared, fixed-height box.
  clustered?: boolean;
  // A long, unbranched run of single-word stages uses content-sized cards.
  compactChain?: boolean;
  // Other visible source cards feeding this same descendant subtree.
  sharedParents?: string[];
  // Extra vertical room supplied only when the subtree fits the viewport.
  breathingRoom?: number;
  // Vertical growth away from the PIE root; the preview occupies the inward edge.
  outwardY?: 1 | -1;
}

// A head opened in a rack, and the blobs that come off it.
export interface LaneSpec {
  hub: string;
  // Keep the remaining tray inward and each new click directly beside it.
  preview?: boolean;
  revealed?: number;
  // The remaining preview box can be taller than a one-line source card.
  height?: number;
  // How far down its blob the head's row is centred, once measured.
  mid: number | undefined;
  blobs: BlobSpec[];
  // A head with one word under it: its one blob is that word alone, and
  // stands level with the head's row, just beside it.
  inline?: boolean;
}

export interface RootSpec {
  id: string;
  r: number;
  // In the order they go round.
  blobs: BlobSpec[];
}

export interface Layout {
  // The root disc's centre; for a head in a rack, where its line leaves the
  // blob.
  hubs: Map<string, Point>;
  // Every blob's own box.
  blobs: Map<string, Rect>;
  // The room each of the root's blobs holds, with everything racked off it.
  seats: Map<string, Rect>;
  // Each blob's outward direction, folded or opened: right (1) or left (-1).
  sides: Map<string, 1 | -1>;
  // Individual cards inside an opened proto-language column.
  sources: Map<string, Rect>;
}

const DEG = Math.PI / 180;
// Clear space kept between any two things, and from the root's rim to its
// blobs.
const GAP = 26;
const REACH = 64;
// How far a blob moves out each time it does not fit, how far round, and how
// far out it is looked for before the frame is given up.
const STEP = 24;
const TURN = 5 * DEG;
const FAR = 1800;
// The root's first blob sits upper left; the rest follow clockwise.
const START = -150 * DEG;
// In a rack: the run of line from the blob to its lanes, the space between
// one head's blobs, and the space between one head's lane and the next's.
const RUN = 112;
const LANE_GAP = 14;
const BETWEEN = 52;
// Horizontal room between a card and the next card out: a head's one word
// stands this far beside its row, and cables make their first turn here.
export const CARD_GAP = 44;
// The same room between the compact cards of a long single-word lineage.
export const COMPACT_GAP = 24;
export const gapAfter = (compact: boolean) => compact ? COMPACT_GAP : CARD_GAP;
// How far outside a column of source cards their shared stem runs.
export const STEM_GUTTER = 24;
// A card's width before content measurement narrows or widens it
// (`sizing.ts`), and a folded card's with four words or fewer. The
// stylesheet reads them as `--cst-card-width` and `--cst-narrow-width`.
export const CARD_WIDTH = 320;
export const NARROW_CARD_WIDTH = 250;
export const SOURCE_HEIGHT = 38;
export const LANGUAGE_HEADING_SPACE = 36;
// A 20px heading has 16px below it and at least the same room above it.
const blobGap = (a: BlobSpec, b?: BlobSpec) => b && a.stage !== b.stage ? BETWEEN : LANE_GAP;

export const center = (r: Rect): Point => ({ x: r.x + r.w / 2, y: r.y + r.h / 2 });

// The smallest box holding all of `rects`; undefined for none.
export function enclosing(rects: Rect[]): Rect | undefined {
  if (!rects.length) return undefined;
  const x = Math.min(...rects.map((r) => r.x));
  const y = Math.min(...rects.map((r) => r.y));
  return {
    x, y,
    w: Math.max(...rects.map((r) => r.x + r.w)) - x,
    h: Math.max(...rects.map((r) => r.y + r.h)) - y,
  };
}

// How far a box reaches from its centre in a direction.
const extent = (s: Size, angle: number) =>
  (Math.abs(Math.cos(angle)) * s.w) / 2 + (Math.abs(Math.sin(angle)) * s.h) / 2;

const around = (c: Point, s: Size): Rect => ({ x: c.x - s.w / 2, y: c.y - s.h / 2, ...s });

const apart = (a: Rect, b: Rect) =>
  a.x >= b.x + b.w + GAP || b.x >= a.x + a.w + GAP || a.y >= b.y + b.h + GAP || b.y >= a.y + a.h + GAP;

// The height of blobs standing one under another.
function stacked(blobs: BlobSpec[], extra = 0): number {
  const measured = blobs.flatMap((b) => {
    const size = whole(b);
    return size ? [{ blob: b, height: size.h }] : [];
  });
  return measured.reduce((sum, b, i) => sum + b.height + (i ? blobGap(measured[i - 1].blob, b.blob) + extra : 0), 0);
}

// The height a rack's lanes take, one under another: each as tall as its
// head's blobs.
function lanesHeight(lanes: LaneSpec[], extra = 0): number {
  const heights = lanes.map((l) => stacked(l.blobs, extra)).filter((h) => h > 0);
  return heights.reduce((sum, h) => sum + h, 0) + (BETWEEN + extra) * Math.max(0, heights.length - 1);
}

// Where a rack's lanes stand, down from the top of its blob: each lane's
// top (none for a lane with nothing measured), and how far the lanes reach
// above the blob's top (`top`, zero or less) and down (`bottom`, the blob's
// height or more). Lanes stand one under another, centred on the blob. A head
// with one word has it level with its own row instead; the lanes of the rows
// above those heads then stand above their words, and the rest below, so the
// caller keeps such heads together.
function lanesAt(b: BlobSpec): { tops: (number | undefined)[]; mids: number[]; top: number; bottom: number } {
  const size = b.size!;
  const lanes = b.lanes ?? [];
  const extra = b.breathingRoom ?? 0, between = BETWEEN + extra;
  const tall = lanes.map((l) => stacked(l.blobs, extra));
  const tops: (number | undefined)[] = lanes.map(() => undefined);
  const mids = lanes.map((l) => l.mid ?? size.h / 2);
  if (b.clustered) {
    let y = 0;
    lanes.forEach((lane, i) => {
      const height = Math.max(lane.height ?? SOURCE_HEIGHT, tall[i]);
      mids[i] = y + height / 2;
      tops[i] = y + (height - tall[i]) / 2;
      y += height + between;
    });
    return { tops, mids, top: 0, bottom: Math.max(SOURCE_HEIGHT, y - between) };
  }
  const level = lanes.flatMap((l, i) => (l.inline && tall[i] ? [i] : []));
  for (const i of level) tops[i] = (lanes[i].mid ?? size.h / 2) - tall[i] / 2;
  const run = (from: number, indexes: number[]) => {
    let y = from;
    for (const i of indexes) {
      tops[i] = y;
      y += tall[i] + between;
    }
  };
  const rest = lanes.flatMap((l, i) => (!l.inline && tall[i] ? [i] : []));
  if (!level.length) {
    run((size.h - lanesHeight(lanes, extra)) / 2, rest);
  } else {
    const above = rest.filter((i) => i < level[0]);
    const below = rest.filter((i) => i > level[0]);
    const bandTop = Math.min(...level.map((i) => tops[i]!));
    const bandBottom = Math.max(...level.map((i) => tops[i]! + tall[i]));
    run(bandTop - above.reduce((sum, i) => sum + tall[i] + between, 0), above);
    run(bandBottom + between, below);
  }
  const placed = tops.flatMap((t, i) => (t === undefined ? [] : [{ t, b: t + tall[i] }]));
  return {
    tops,
    mids,
    top: Math.min(0, ...placed.map((p) => p.t)),
    bottom: Math.max(size.h, ...placed.map((p) => p.b)),
  };
}

// The room a blob takes with everything racked off it.
export function whole(b: BlobSpec): Size | undefined {
  if (!b.size) return undefined;
  const reach = Math.max(
    0,
    ...(b.lanes ?? []).flatMap((l) =>
      l.blobs.flatMap((k) => {
        const w = whole(k)?.w;
        return w ? [(k.gap ?? (l.inline ? CARD_GAP : RUN)) + w] : [];
      })
    )
  );
  if (!reach && !b.clustered) return b.size;
  const at = lanesAt(b);
  return { w: b.size.w + reach, h: at.bottom - at.top };
}

// Spend spare viewport height on separation, capped at 40 extra pixels.
// Estimate the complete subtree before adding space, so dense trees keep
// their compact geometry and the camera never drives a spacing feedback loop.
export function breathe(b: BlobSpec, availableHeight: number): BlobSpec {
  const node = { ...b, lanes: b.lanes?.map((l) => ({ ...l,
    blobs: l.blobs.map((child) => breathe(child, availableHeight)),
  })) };
  const baseline = whole(node)?.h ?? 0;
  const seams = Math.max(0, (node.lanes?.length ?? 0) - 1)
    + (node.lanes ?? []).reduce((sum, l) => sum + Math.max(0, l.blobs.length - 1), 0);
  const extra = seams ? Math.min(40, Math.max(0, (availableHeight * 0.7 - baseline) / seams)) : 0;
  return { ...node, breathingRoom: extra };
}

// `frame`: the room the root and its blobs have, centred on the root.
// `before`: the layout this one follows, whose seats are kept where they can.
export function layout(root: RootSpec, frame?: Size, before?: Layout): Layout {
  const hubs = new Map<string, Point>();
  const blobs = new Map<string, Rect>();
  const seats = new Map<string, Rect>();
  const sides = new Map<string, 1 | -1>();
  const sources = new Map<string, Rect>();
  const at: Point = { x: 0, y: 0 };
  hubs.set(root.id, at);
  const disc = around(at, { w: root.r * 2, h: root.r * 2 });
  const free = (r: Rect, others: Rect[]) => apart(r, disc) && others.every((p) => apart(r, p));

  // A blob in the room found for it: its own box and, for a rack, its lanes
  // on the side away from `fromX`.
  const settle = (b: BlobSpec, room: Rect, fromX: number, endX: number) => {
    const size = b.size!;
    // Folded previews need the same direction as opened racks for their
    // source/descendant columns and expansion controls.
    const side = room.x + room.w / 2 >= fromX ? 1 : -1;
    sides.set(b.id, side);
    if (!b.lanes?.length) {
      blobs.set(b.id, b.terminal ? { ...room, x: side > 0 ? endX - room.w : endX } : room);
      return;
    }
    // Ordinary racks keep their own size. A proto rack's invisible column
    // stretches to place each separate source card beside its own subtree.
    const at = lanesAt(b);
    const box: Rect = { x: side > 0 ? room.x : room.x + room.w - size.w, y: room.y - at.top,
      ...size, ...(b.clustered ? { h: at.bottom } : {}) };
    blobs.set(b.id, box);
    b.lanes.forEach((lane, i) => {
      const leaves = { x: side > 0 ? box.x + box.w : box.x, y: box.y + at.mids[i] };
      hubs.set(lane.hub, leaves);
      if (b.clustered) {
        const h = lane.height ?? SOURCE_HEIGHT;
        sources.set(lane.hub, { x: box.x, y: leaves.y - h / 2, w: box.w, h });
      }
      let top = at.tops[i];
      if (top === undefined) return;
      const kids = lane.blobs.filter((kid) => whole(kid));
      for (const [index, kid] of kids.entries()) {
        const s = whole(kid);
        if (!s) continue;
        const run = kid.gap ?? (lane.inline ? CARD_GAP : RUN);
        settle(kid, { x: side > 0 ? box.x + box.w + run : box.x - run - s.w, y: box.y + top, ...s }, leaves.x, endX);
        top += s.h + blobGap(kid, kids[index + 1]) + (b.breathingRoom ?? 0);
      }
    });
  };

  // A blob against the root at an angle, `out` further than snug.
  const seat = (size: Size, angle: number, out: number): Rect => {
    // Anchor the inward edge, not the centre of the complete subtree.
    // Wider descendants grow outward; they cannot push a family's stem
    // across PIE into the neighbouring family's departure corridor.
    const d = root.r + REACH + Math.abs(Math.sin(angle)) * size.h / 2 + out;
    const side = Math.cos(angle) >= 0 ? 1 : -1;
    return around({ x: Math.cos(angle) * d + side * size.w / 2, y: Math.sin(angle) * d }, size);
  };
  // Each blob has a fixed direction from the root and moves farther out
  // along it when more room is needed. Within `room`, a
  // box centred on the root, or not at all when it has no seat in it.
  const n = root.blobs.length;
  // Root branches keep distinct directions. Searching the whole circle
  // for a smaller packing can put one sibling behind another.
  const sector = n > 1 ? 0 : Math.PI;
  const inSector = (i: number, r: Rect) => {
    if (n <= 1) return true;
    const turn = START + (i * 2 * Math.PI) / n;
    const inwardX = Math.cos(turn) >= 0 ? r.x : r.x + r.w;
    const angle = Math.atan2(r.y + r.h / 2, inwardX);
    return Math.abs(Math.atan2(Math.sin(angle - turn), Math.cos(angle - turn))) <= sector + 1e-6;
  };
  const find = (i: number, size: Size, room: Size | null, mine: Rect[]): Rect | null => {
    const fits = (r: Rect) =>
      (!room ||
        (Math.abs(r.x + r.w / 2) + r.w / 2 <= room.w / 2 && Math.abs(r.y + r.h / 2) + r.h / 2 <= room.h / 2)) &&
      free(r, mine);
    const far = room ? Math.hypot(room.w, room.h) / 2 : FAR;
    const turn = START + (i * 2 * Math.PI) / n;
    for (let out = 0; out <= far; out += STEP)
      for (let off = 0; off <= sector + 1e-6; off += TURN)
        for (const angle of off === 0 ? [turn] : [turn + off, turn - off]) {
          const r = seat(size, angle, out);
          if (fits(r)) return r;
        }
    if (room) return null;
    for (let t = root.r + REACH + extent(size, turn); ; t += STEP) {
      const r = seat(size, turn, t);
      if (fits(r)) return r;
    }
  };
  const sizes = root.blobs.map(whole);
  const ring = (room: Size | null): (Rect | null)[] | null => {
    const mine: Rect[] = [];
    const found: (Rect | null)[] = [];
    for (const [i, size] of sizes.entries()) {
      const r = size ? find(i, size, room, mine) : null;
      if (size && !r) return null;
      if (r) mine.push(r);
      found.push(r);
    }
    return found;
  };

  let found: (Rect | null)[] | null = null;
  if (before?.hubs.has(root.id)) {
    // Blobs that have not changed size keep their seats, and one that has
    // (or is new) finds a seat round them: only the blob the reader opened
    // moves.
    const mine: Rect[] = [];
    found = root.blobs.map((blob, i) => {
      const size = sizes[i];
      const old = before.seats.get(blob.id);
      const keeps = size && old && old.w === size.w && old.h === size.h && inSector(i, old) && free(old, mine);
      if (keeps) mine.push(old);
      return keeps ? old : null;
    });
    for (const [i, size] of sizes.entries()) {
      if (found[i] || !size) continue;
      const r = find(i, size, null, mine)!;
      mine.push(r);
      found[i] = r;
    }
  } else {
    // At first the blobs are packed into the shape of the screen, in the
    // least room that takes them all, so the first view fits it as large as
    // can be.
    if (frame)
      for (let scale = 1; !found && scale <= 3; scale += 0.06)
        found = ring({ w: frame.w * scale, h: frame.h * scale });
    found ??= ring(null)!;
  }
  root.blobs.forEach((blob, i) => {
    const r = found[i];
    if (!r) return;
    seats.set(blob.id, r);
    settle(blob, r, at.x, r.x + r.w / 2 >= at.x ? r.x + r.w : r.x);
  });
  // Shared destinations cannot be positioned as one arbitrary parent's
  // child. Repack each affected sibling frontier around all incoming roots.
  const repack = (b: BlobSpec) => {
    const children = (b.lanes ?? []).flatMap((lane) => lane.blobs.map((child) => ({ child, owner: lane.hub })));
    if (children.some(({ child }) => child.sharedParents?.length)) {
      const groups = children.flatMap(({ child, owner }) => {
        const box = blobs.get(child.id), size = whole(child);
        if (!box || !size) return [];
        const parents = [owner, ...(child.sharedParents ?? [])].flatMap((id) => hubs.get(id) ?? []);
        const wanted = parents.reduce((sum, p) => sum + p.y, 0) / parents.length;
        return [{ child, top: box.y + (child.lanes?.length ? lanesAt(child).top : 0), height: size.h, wanted }];
      }).sort((a, b) => a.wanted - b.wanted);
      const total = groups.reduce((sum, g) => sum + g.height, 0) + BETWEEN * Math.max(0, groups.length - 1);
      const center = groups.reduce((sum, g) => sum + g.wanted * g.height, 0) / Math.max(1, total - BETWEEN * Math.max(0, groups.length - 1));
      const start = Math.min(...groups.map((g) => g.top));
      const end = Math.max(...groups.map((g) => g.top + g.height));
      let top = Math.max(start, Math.min(end - total, center - total / 2));
      const move = (child: BlobSpec, dy: number) => {
        const box = blobs.get(child.id);
        if (box) blobs.set(child.id, { ...box, y: box.y + dy });
        for (const lane of child.lanes ?? []) {
          const p = hubs.get(lane.hub), source = sources.get(lane.hub);
          if (p) hubs.set(lane.hub, { ...p, y: p.y + dy });
          if (source) sources.set(lane.hub, { ...source, y: source.y + dy });
          lane.blobs.forEach((next) => move(next, dy));
        }
      };
      for (const group of groups) {
        move(group.child, top - group.top);
        top += group.height + BETWEEN;
      }
    }
    children.forEach(({ child }) => repack(child));
  };
  root.blobs.forEach(repack);
  return { hubs, blobs, seats, sides, sources };
}

// The box that holds the root and its blobs, for fitting them on screen.
export function bounds(at: Layout, root: string, r: number, blobIds: string[]): Rect | null {
  const c = at.hubs.get(root);
  if (!c) return null;
  return enclosing([
    around(c, { w: r * 2, h: r * 2 }),
    ...blobIds.flatMap((id) => at.seats.get(id) ?? at.blobs.get(id) ?? []),
  ])!;
}

// Where a line from a box's centre towards a point crosses its edge.
export function edgeToward(r: Rect, p: Point): Point {
  const c = center(r);
  const dx = p.x - c.x;
  const dy = p.y - c.y;
  if (dx === 0 && dy === 0) return c;
  const t = Math.min(
    dx === 0 ? Infinity : r.w / 2 / Math.abs(dx),
    dy === 0 ? Infinity : r.h / 2 / Math.abs(dy)
  );
  return { x: c.x + dx * t, y: c.y + dy * t };
}

export interface Link {
  d: string;
  end: Point;
  // Marked too, where the line leaves a rack.
  start?: Point;
}

const f = (n: number) => n.toFixed(1);

// Each sibling has its own departure point, even when a rack's visible
// box is off-centre within the space reserved for all of its descendants.
export function rootDirection(index: number, count: number): number {
  return START + index * 2 * Math.PI / count;
}

// Meet a branch stem before its first card (or after its last card), rather
// than routing around an invisible column to reach a point inside it.
export function stemArrival(c: Point, x: number, sources: Rect[]): Point {
  const top = Math.min(...sources.map((s) => s.y)) - 32;
  const bottom = Math.max(...sources.map((s) => s.y + s.h)) + 32;
  return { x, y: Math.max(top, Math.min(bottom, c.y)) };
}

// How far cables keep from the cards they pass.
const CLEARANCE = 16;
const inflate = (r: Rect, by: number): Rect => ({ x: r.x - by, y: r.y - by, w: r.w + 2 * by, h: r.h + 2 * by });

// Whether the segment a–b passes through the open interior of `rect`;
// running along its edge does not count.
function crossesInterior(a: Point, b: Point, rect: Rect): boolean {
  let lo = 0, hi = 1;
  for (const [p, d, min, max] of [[a.x, b.x - a.x, rect.x, rect.x + rect.w], [a.y, b.y - a.y, rect.y, rect.y + rect.h]]) {
    if (Math.abs(d) < 1e-9) {
      if (p <= min + 1e-6 || p >= max - 1e-6) return false;
    } else {
      const t0 = (min - p) / d, t1 = (max - p) / d;
      lo = Math.max(lo, Math.min(t0, t1));
      hi = Math.min(hi, Math.max(t0, t1));
    }
  }
  return lo + 1e-6 < hi;
}

// A gently bowed line from the root's rim to a blob's edge.
export function join(c: Point, r: number, box: Rect, obstacles: Rect[] = [], direction?: number, stem = false): Link {
  const to = center(box);
  const angle = direction ?? Math.atan2(to.y - c.y, to.x - c.x);
  const start = { x: c.x + Math.cos(angle) * r, y: c.y + Math.sin(angle) * r };
  const end = edgeToward(box, c);
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const bow = Math.sin(2 * Math.atan2(dy, dx)) >= 0 ? 0.14 : -0.14;
  const between = (value: number, a: number, b: number) => Math.max(Math.min(a, b), Math.min(Math.max(a, b), value));
  const cx = between((start.x + end.x) / 2 - dy * bow, start.x, end.x);
  const cy = between((start.y + end.y) / 2 + dx * bow, start.y, end.y);
  const reach = Math.min(80, Math.hypot(dx, dy) / 3);
  // Keep both control points between the endpoints. Radial tangents with a
  // fixed reach can overshoot short connections and produce a backwards hook.
  const control1 = { x: start.x + Math.sign(dx) * Math.min(Math.abs(dx) / 2, reach),
    y: start.y + Math.sign(dy) * Math.min(Math.abs(dy) / 2, reach) };
  const control2 = { x: end.x, y: end.y - Math.sign(dy) * Math.min(Math.abs(dy) / 2, reach) };
  // A tall sibling can hide this cable and make it appear to originate
  // there. Route around the sibling's entire reserved space, not through it,
  // with the same orthogonal router every other cable uses.
  const blocks = obstacles.map((b) => inflate(b, CLEARANCE));
  let prev = start;
  for (let i = 1; i <= 32; i++) {
    const t = i / 32, u = 1 - t;
    const p = stem ? {
      x: u ** 3 * start.x + 3 * u * u * t * control1.x + 3 * u * t * t * control2.x + t ** 3 * end.x,
      y: u ** 3 * start.y + 3 * u * u * t * control1.y + 3 * u * t * t * control2.y + t ** 3 * end.y,
    } : { x: u * u * start.x + 2 * u * t * cx + t * t * end.x, y: u * u * start.y + 2 * u * t * cy + t * t * end.y };
    if (blocks.some((b) => crossesInterior(prev, p, b))) return { d: cableRouter(obstacles)(start, end).d, end };
    prev = p;
  }
  return { d: stem
    ? `M ${f(start.x)} ${f(start.y)} C ${f(control1.x)} ${f(control1.y)} ${f(control2.x)} ${f(control2.y)} ${f(end.x)} ${f(end.y)}`
    : `M ${f(start.x)} ${f(start.y)} Q ${f(cx)} ${f(cy)} ${f(end.x)} ${f(end.y)}`, end };
}

// Round only the corners of an orthogonal route; never draw a diagonal
// straight segment. The obstacle clearance leaves room for these curves.
function roundedRoute(points: Point[]): string {
  const path = points.filter((p, i) => !i || p.x !== points[i - 1].x || p.y !== points[i - 1].y);
  for (let i = path.length - 2; i > 0; i--) {
    const [a, b, c] = [path[i - 1], path[i], path[i + 1]];
    if ((a.x === b.x && b.x === c.x) || (a.y === b.y && b.y === c.y)) path.splice(i, 1);
  }
  let d = `M ${f(path[0].x)} ${f(path[0].y)}`;
  for (let i = 1; i < path.length - 1; i++) {
    const [a, b, c] = [path[i - 1], path[i], path[i + 1]];
    const before = Math.abs(b.x - a.x) + Math.abs(b.y - a.y);
    const after = Math.abs(c.x - b.x) + Math.abs(c.y - b.y);
    const radius = Math.min(12, before / 2, after / 2);
    const enter = { x: b.x + (a.x - b.x) * radius / before, y: b.y + (a.y - b.y) * radius / before };
    const leave = { x: b.x + (c.x - b.x) * radius / after, y: b.y + (c.y - b.y) * radius / after };
    d += ` L ${f(enter.x)} ${f(enter.y)} Q ${f(b.x)} ${f(b.y)} ${f(leave.x)} ${f(leave.y)}`;
  }
  const last = path.at(-1)!;
  return `${d} L ${f(last.x)} ${f(last.y)}`;
}


// Several sources feeding one destination are one connection tree, not
// independent cables. Join them in the destination's gutter, then enter once.
export function sharedWires(from: Point[], to: Point, cards: Rect[], lane = 0): Link[] {
  const side = from.reduce((sum, p) => sum + p.x - to.x, 0) >= 0 ? 1 : -1;
  const distances = from.map((p) => side * (p.x - to.x)).filter((d) => d > 0);
  const closest = distances.length ? Math.min(...distances) : 48;
  const x = to.x + side * Math.max(closest - STEM_GUTTER - lane * 12, closest / 2);
  const top = Math.min(to.y, ...from.map((p) => p.y));
  const bottom = Math.max(to.y, ...from.map((p) => p.y));
  const route = cableRouter(cards);
  const lines = from.map((p) => route(p, { x, y: p.y }));
  if (top !== bottom) lines.push({ d: roundedRoute([{ x, y: top }, { x, y: bottom }]), end: { x, y: bottom } });
  lines.push(route({ x, y: to.y }, to));
  return lines;
}

// One router per drawing pass. Independent connections get separate vertical
// lanes; genuine branch stems are still drawn once by the caller. Routing uses
// visible card geometry, so it also works after viewport-driven compression.
export function cableRouter(cards: Rect[], stems: { x: number; top: number; bottom: number }[] = []): (from: Point, to: Point) => Link {
  const occupied = [...stems];
  const runs: { a: Point; b: Point }[] = stems.map((s) => ({ a: { x: s.x, y: s.top }, b: { x: s.x, y: s.bottom } }));
  const contains = (r: Rect, p: Point) => p.x >= r.x - 0.1 && p.x <= r.x + r.w + 0.1
    && p.y >= r.y - 0.1 && p.y <= r.y + r.h + 0.1;
  return (from, to) => {
    const blocks = cards.filter((r) => !contains(r, from) && !contains(r, to))
      .map((r) => inflate(r, CLEARANCE));
    const clear = (a: Point, b: Point) => !blocks.some((r) => crossesInterior(a, b, r));
    const laneCost = (a: Point, b: Point) => a.x !== b.x || a.y === b.y ? 0 : occupied.reduce((cost, lane) =>
      cost + (Math.abs(lane.x - a.x) < 10 && Math.max(a.y, b.y) > lane.top && Math.min(a.y, b.y) < lane.bottom ? 10000 : 0), 0);
    const crossingCost = (a: Point, b: Point) => runs.reduce((cost, run) => {
      const horizontal = a.y === b.y, otherHorizontal = run.a.y === run.b.y;
      if (horizontal === otherHorizontal) return cost;
      const h = horizontal ? { a, b } : run, v = horizontal ? run : { a, b };
      return cost + (v.a.x > Math.min(h.a.x, h.b.x) + 0.1 && v.a.x < Math.max(h.a.x, h.b.x) - 0.1
        && h.a.y > Math.min(v.a.y, v.b.y) + 0.1 && h.a.y < Math.max(v.a.y, v.b.y) - 0.1 ? 5000 : 0);
    }, 0);
    const side = Math.sign(to.x - from.x) || 1;
    const preferred = from.x + side * Math.min(CARD_GAP, Math.abs(to.x - from.x) / 2);
    const xs = [...new Set([preferred, from.x + side * CLEARANCE, to.x - side * CLEARANCE,
      ...Array.from({ length: 9 }, (_, i) => preferred + (i - 4) * 12),
      ...blocks.flatMap((r) => [r.x, r.x + r.w])])];
    let best: Point[] | undefined;
    let score = Infinity;
    for (const x of xs) {
      const path = [from, { x, y: from.y }, { x, y: to.y }, to];
      if (!path.slice(1).every((p, i) => clear(path[i], p))) continue;
      const cost = Math.abs(x - preferred) + laneCost(path[1], path[2])
        + path.slice(1).reduce((sum, p, i) => sum + crossingCost(path[i], p), 0);
      if (cost < score) { best = path; score = cost; }
    }
    if (!best || score >= 5000) {
      // A card spans the whole direct corridor. Walk a rectilinear visibility
      // grid around card edges, penalizing bends and previously used lanes.
      const nearby = runs.filter(({ a, b }) => Math.max(a.x, b.x) >= Math.min(from.x, to.x) - 40
        && Math.min(a.x, b.x) <= Math.max(from.x, to.x) + 40);
      const gridX = [...new Set([from.x, to.x, ...xs,
        ...nearby.flatMap(({ a, b }) => [Math.min(a.x, b.x) - CLEARANCE, Math.max(a.x, b.x) + CLEARANCE])])].sort((a, b) => a - b);
      const gridY = [...new Set([from.y, to.y, ...blocks.flatMap((r) => [r.y, r.y + r.h]),
        ...nearby.flatMap(({ a, b }) => [Math.min(a.y, b.y) - CLEARANCE, Math.max(a.y, b.y) + CLEARANCE])])].sort((a, b) => a - b);
      const key = (x: number, y: number, dir: number) => `${x},${y},${dir}`;
      const start = { x: gridX.indexOf(from.x), y: gridY.indexOf(from.y), dir: 0, cost: 0, path: [from] };
      const queue = [start];
      const costs = new Map<string, number>();
      while (queue.length) {
        queue.sort((a, b) => b.cost - a.cost);
        const here = queue.pop()!;
        if (here.cost > (costs.get(key(here.x, here.y, here.dir)) ?? Infinity)) continue;
        const a = { x: gridX[here.x], y: gridY[here.y] };
        if (a.x === to.x && a.y === to.y) { best = here.path; break; }
        for (const [dx, dy, dir] of [[1, 0, 1], [-1, 0, 1], [0, 1, 2], [0, -1, 2]]) {
          const x = here.x + dx, y = here.y + dy;
          if (x < 0 || y < 0 || x >= gridX.length || y >= gridY.length) continue;
          const b = { x: gridX[x], y: gridY[y] };
          if (!clear(a, b)) continue;
          const cost = here.cost + Math.abs(a.x - b.x) + Math.abs(a.y - b.y)
            + (here.dir && dir !== here.dir ? 24 : 0) + laneCost(a, b) + crossingCost(a, b);
          const id = key(x, y, dir);
          if (cost >= (costs.get(id) ?? Infinity)) continue;
          costs.set(id, cost);
          queue.push({ x, y, dir, cost, path: [...here.path, b] });
        }
      }
    }
    best ??= [from, { x: preferred, y: from.y }, { x: preferred, y: to.y }, to];
    for (let i = 1; i < best.length; i++) {
      const a = best[i - 1], b = best[i];
      if (a.x !== b.x || a.y !== b.y) runs.push({ a, b });
      if (a.x === b.x && a.y !== b.y) occupied.push({ x: a.x, top: Math.min(a.y, b.y), bottom: Math.max(a.y, b.y) });
    }
    return { d: roundedRoute(best), start: from, end: to };
  };
}
