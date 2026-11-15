// Where the constellation's root and blobs go.
//
// The root is a disc with its blobs round it. A blob whose heads have been
// opened is a rack: the heads are rows in the blob, which keeps its own
// size, and each row's blobs stand in a lane beside it, the lanes one under
// another in the rows' order, so the lines from heads to blobs fan out side
// by side. A head with one word under it has that word level with its row
// instead, at the end of a short straight line. A lane's blob may be a rack
// in turn, and so on outwards.

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
}

// A head opened in a rack, and the blobs that come off it.
export interface LaneSpec {
  hub: string;
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
  // Which way a rack's lanes run from its blob: right (1) or left (-1).
  sides: Map<string, 1 | -1>;
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
const BETWEEN = 26;
// The run of line from a head's row to its one word.
const SHORT = 44;

export const center = (r: Rect): Point => ({ x: r.x + r.w / 2, y: r.y + r.h / 2 });

// How far a box reaches from its centre in a direction.
const extent = (s: Size, angle: number) =>
  (Math.abs(Math.cos(angle)) * s.w) / 2 + (Math.abs(Math.sin(angle)) * s.h) / 2;

const around = (c: Point, s: Size): Rect => ({ x: c.x - s.w / 2, y: c.y - s.h / 2, ...s });

const apart = (a: Rect, b: Rect) =>
  a.x >= b.x + b.w + GAP || b.x >= a.x + a.w + GAP || a.y >= b.y + b.h + GAP || b.y >= a.y + a.h + GAP;

// The height of blobs standing one under another.
function stacked(blobs: BlobSpec[]): number {
  const heights = blobs.flatMap((b) => whole(b)?.h ?? []);
  return heights.reduce((sum, h) => sum + h, 0) + LANE_GAP * Math.max(0, heights.length - 1);
}

// The height a rack's lanes take, one under another: each as tall as its
// head's blobs.
function lanesHeight(lanes: LaneSpec[]): number {
  const heights = lanes.map((l) => stacked(l.blobs)).filter((h) => h > 0);
  return heights.reduce((sum, h) => sum + h, 0) + BETWEEN * Math.max(0, heights.length - 1);
}

// Where a rack's lanes stand, down from the top of its blob: each lane's
// top (none for a lane with nothing measured), and how far the lanes reach
// above the blob's top (`top`, zero or less) and down (`bottom`, the blob's
// height or more). Lanes stand one under another, centred on the blob. A head
// with one word has it level with its own row instead; the lanes of the rows
// above those heads then stand above their words, and the rest below, so the
// caller keeps such heads together.
function lanesAt(b: BlobSpec): { tops: (number | undefined)[]; top: number; bottom: number } {
  const size = b.size!;
  const lanes = b.lanes ?? [];
  const tall = lanes.map((l) => stacked(l.blobs));
  const tops: (number | undefined)[] = lanes.map(() => undefined);
  const level = lanes.flatMap((l, i) => (l.inline && tall[i] ? [i] : []));
  for (const i of level) tops[i] = (lanes[i].mid ?? size.h / 2) - tall[i] / 2;
  const run = (from: number, indexes: number[]) => {
    let y = from;
    for (const i of indexes) {
      tops[i] = y;
      y += tall[i] + BETWEEN;
    }
  };
  const rest = lanes.flatMap((l, i) => (!l.inline && tall[i] ? [i] : []));
  if (!level.length) {
    run((size.h - lanesHeight(lanes)) / 2, rest);
  } else {
    const above = rest.filter((i) => i < level[0]);
    const below = rest.filter((i) => i > level[0]);
    const bandTop = Math.min(...level.map((i) => tops[i]!));
    const bandBottom = Math.max(...level.map((i) => tops[i]! + tall[i]));
    run(bandTop - above.reduce((sum, i) => sum + tall[i] + BETWEEN, 0), above);
    run(bandBottom + BETWEEN, below);
  }
  const placed = tops.flatMap((t, i) => (t === undefined ? [] : [{ t, b: t + tall[i] }]));
  return {
    tops,
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
        return w ? [(k.gap ?? (l.inline ? SHORT : RUN)) + w] : [];
      })
    )
  );
  if (!reach) return b.size;
  const at = lanesAt(b);
  return { w: b.size.w + reach, h: at.bottom - at.top };
}

// `frame`: the room the root and its blobs have, centred on the root.
// `before`: the layout this one follows, whose seats are kept where they can.
export function layout(root: RootSpec, frame?: Size, before?: Layout): Layout {
  const hubs = new Map<string, Point>();
  const blobs = new Map<string, Rect>();
  const seats = new Map<string, Rect>();
  const sides = new Map<string, 1 | -1>();
  const at: Point = { x: 0, y: 0 };
  hubs.set(root.id, at);
  const disc = around(at, { w: root.r * 2, h: root.r * 2 });
  const free = (r: Rect, others: Rect[]) => apart(r, disc) && others.every((p) => apart(r, p));

  // A blob in the room found for it: its own box and, for a rack, its lanes
  // on the side away from `fromX`.
  const settle = (b: BlobSpec, room: Rect, fromX: number, endX: number) => {
    const size = b.size!;
    if (!b.lanes?.length) {
      const right = room.x + room.w / 2 >= fromX;
      blobs.set(b.id, b.terminal ? { ...room, x: right ? endX - room.w : endX } : room);
      return;
    }
    // The blob keeps its own size; the lanes stand beside it (`lanesAt`), in
    // the order of their heads' rows, so the lines fan out from the rows
    // without crossing.
    const side = room.x + room.w / 2 >= fromX ? 1 : -1;
    const at = lanesAt(b);
    const box: Rect = { x: side > 0 ? room.x : room.x + room.w - size.w, y: room.y - at.top, ...size };
    blobs.set(b.id, box);
    sides.set(b.id, side);
    b.lanes.forEach((lane, i) => {
      const leaves = { x: side > 0 ? box.x + box.w : box.x, y: box.y + (lane.mid ?? size.h / 2) };
      hubs.set(lane.hub, leaves);
      let top = at.tops[i];
      if (top === undefined) return;
      for (const kid of lane.blobs) {
        const s = whole(kid);
        if (!s) continue;
        const run = kid.gap ?? (lane.inline ? SHORT : RUN);
        settle(kid, { x: side > 0 ? box.x + box.w + run : box.x - run - s.w, y: box.y + top, ...s }, leaves.x, endX);
        top += s.h + LANE_GAP;
      }
    });
  };

  // A blob against the root at an angle, `out` further than snug.
  const seat = (size: Size, angle: number, out: number): Rect => {
    const d = root.r + REACH + extent(size, angle) + out;
    return around({ x: Math.cos(angle) * d, y: Math.sin(angle) * d }, size);
  };
  // Each blob has its turn round the root and takes the free seat nearest
  // it, moving out a little at a time when there is none. Within `room`, a
  // box centred on the root, or not at all when it has no seat in it.
  const n = root.blobs.length;
  const find = (i: number, size: Size, room: Size | null, mine: Rect[]): Rect | null => {
    const fits = (r: Rect) =>
      (!room ||
        (Math.abs(r.x + r.w / 2) + r.w / 2 <= room.w / 2 && Math.abs(r.y + r.h / 2) + r.h / 2 <= room.h / 2)) &&
      free(r, mine);
    const far = room ? Math.hypot(room.w, room.h) / 2 : FAR;
    const turn = START + (i * 2 * Math.PI) / n;
    for (let out = 0; out <= far; out += STEP)
      for (let off = 0; off <= Math.PI; off += TURN)
        for (const angle of off === 0 ? [turn] : [turn + off, turn - off]) {
          const r = seat(size, angle, out);
          if (fits(r)) return r;
        }
    if (room) return null;
    for (let t = root.r + REACH + extent(size, turn); ; t += STEP) {
      const r = around({ x: Math.cos(turn) * t, y: Math.sin(turn) * t }, size);
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
      const keeps = size && old && old.w === size.w && old.h === size.h && free(old, mine);
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
  return { hubs, blobs, seats, sides };
}

// The box that holds the root and its blobs, for fitting them on screen.
export function bounds(at: Layout, root: string, r: number, blobIds: string[]): Rect | null {
  const c = at.hubs.get(root);
  if (!c) return null;
  const rects = [
    around(c, { w: r * 2, h: r * 2 }),
    ...blobIds.map((id) => at.seats.get(id) ?? at.blobs.get(id)).filter((b) => !!b),
  ];
  const x0 = Math.min(...rects.map((b) => b.x));
  const y0 = Math.min(...rects.map((b) => b.y));
  const x1 = Math.max(...rects.map((b) => b.x + b.w));
  const y1 = Math.max(...rects.map((b) => b.y + b.h));
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
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

// A gently bowed line from the root's rim to a blob's edge.
export function join(c: Point, r: number, box: Rect): Link {
  const to = center(box);
  const angle = Math.atan2(to.y - c.y, to.x - c.x);
  const start = { x: c.x + Math.cos(angle) * r, y: c.y + Math.sin(angle) * r };
  const end = edgeToward(box, c);
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const bow = Math.sin(2 * Math.atan2(dy, dx)) >= 0 ? 0.14 : -0.14;
  const cx = (start.x + end.x) / 2 - dy * bow;
  const cy = (start.y + end.y) / 2 + dx * bow;
  return { d: `M ${f(start.x)} ${f(start.y)} Q ${f(cx)} ${f(cy)} ${f(end.x)} ${f(end.y)}`, end };
}

// A line that leaves and arrives level: in a rack, from a head's row to the
// side of one of its blobs.
export function wire(from: Point, to: Point, terminal = false): Link {
  const half = (to.x - from.x) / 2;
  // An aligned endpoint can be several columns away. Finish its vertical
  // turn before the first column, then run level past intermediate nodes.
  const turn = Math.sign(half) * RUN;
  if (terminal && Math.abs(half) > RUN / 2) {
    return {
      d: `M ${f(from.x)} ${f(from.y)} C ${f(from.x + turn / 2)} ${f(from.y)} ${f(from.x + turn / 2)} ${f(to.y)} ${f(from.x + turn)} ${f(to.y)} L ${f(to.x)} ${f(to.y)}`,
      end: to,
      start: from,
    };
  }
  return {
    d: `M ${f(from.x)} ${f(from.y)} C ${f(from.x + half)} ${f(from.y)} ${f(to.x - half)} ${f(to.y)} ${f(to.x)} ${f(to.y)}`,
    end: to,
    start: from,
  };
}
