// Where the constellation's hubs and blobs go.
//
// A hub with a disc has its blobs round it: the root's all the way round,
// a later hub's in a fan ahead of it. Hubs are placed in the order they were
// opened, each where the blob it was opened from sat and its own blobs round
// what is already there, so opening a word never rearranges what came before.
//
// A blob whose heads were all opened at once is a rack: the heads are rows
// in the blob, which keeps its own size, and each row's blobs stand in a
// lane beside it, the lanes one under another in the rows' order, so the
// lines from heads to blobs fan out side by side.

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
  // For a blob in a lane: the hub whose disc has taken its place, with the
  // blobs that come off it, which stand in a column beyond the disc. (A disc
  // that takes the place of a blob round another disc is a `HubSpec`, with
  // room to have its own blobs round it.)
  disc?: { hub: string; r: number; blobs: BlobSpec[] };
}

// A head opened in a rack, and the blobs that come off it.
export interface LaneSpec {
  hub: string;
  // How far down its blob the head's row is centred, once measured.
  mid: number | undefined;
  blobs: BlobSpec[];
}

// A hub with a disc.
export interface HubSpec {
  id: string;
  r: number;
  // The blob this hub was opened from, and the hub that blob belongs to; none
  // for the root. The blob held nothing else and is no longer drawn: the disc
  // takes its place.
  from: { hub: string; blob: string } | null;
  // In the order they go round.
  blobs: BlobSpec[];
}

export interface Layout {
  // A disc's centre; for a head in a rack, where its line leaves the blob.
  hubs: Map<string, Point>;
  // Every blob's own box.
  blobs: Map<string, Rect>;
  // The room a disc's blob holds, with everything racked off it.
  seats: Map<string, Rect>;
  // Which way things run on from a rack's blob (by blob) or from a disc in a
  // lane (by hub): right (1) or left (-1).
  sides: Map<string, 1 | -1>;
}

const DEG = Math.PI / 180;
// Clear space kept between any two things, and from a hub's rim to its blobs.
const GAP = 26;
const REACH = 64;
// How far a thing moves out each time it does not fit, how far round, and
// how far out a blob is looked for before the frame is given up.
const STEP = 24;
const TURN = 5 * DEG;
const FAR = 1800;
// The root's first blob sits upper left; the rest follow clockwise.
const ROOT_START = -150 * DEG;
// A later hub's blobs fan out ahead of it, away from where it came from.
const EACH = 64 * DEG;
const FAN = 300 * DEG;
// In a rack: the run of line from the blob to its lanes, the space between
// one head's blobs, and the space between one head's lane and the next's.
const RUN = 112;
const LANE_GAP = 14;
const BETWEEN = 26;

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

// The room a blob takes with everything that runs on from it: a rack's
// lanes, or the disc that has taken its place in a lane and that disc's
// blobs in a column beyond it.
export function whole(b: BlobSpec): Size | undefined {
  if (b.disc) {
    const d = b.disc.r * 2;
    const beyond = Math.max(0, ...b.disc.blobs.map((k) => whole(k)?.w ?? 0));
    return beyond ? { w: d + RUN + beyond, h: Math.max(d, stacked(b.disc.blobs)) } : { w: d, h: d };
  }
  if (!b.size) return undefined;
  const widest = Math.max(0, ...(b.lanes ?? []).flatMap((l) => l.blobs.map((k) => whole(k)?.w ?? 0)));
  if (!widest) return b.size;
  return { w: b.size.w + RUN + widest, h: Math.max(b.size.h, lanesHeight(b.lanes!)) };
}

// `frame`: the room the root and its blobs have, centred on the root.
// `before`: the layout this one follows, whose seats are kept where they can.
export function layout(specs: HubSpec[], frame?: Size, before?: Layout): Layout {
  const hubs = new Map<string, Point>();
  const blobs = new Map<string, Rect>();
  const seats = new Map<string, Rect>();
  const sides = new Map<string, 1 | -1>();
  const placed: Rect[] = [];
  const free = (r: Rect, others: Rect[] = placed) => others.every((p) => apart(r, p));

  // A blob in the room found for it: its own box and, on the side away from
  // `fromX`, what runs on from it: a rack's lanes, or a lane disc's blobs in
  // a column centred on it.
  const settle = (b: BlobSpec, room: Rect, fromX: number) => {
    const side = room.x + room.w / 2 >= fromX ? 1 : -1;
    if (b.disc) {
      const r = b.disc.r;
      const c = { x: side > 0 ? room.x + r : room.x + room.w - r, y: room.y + room.h / 2 };
      hubs.set(b.disc.hub, c);
      sides.set(b.disc.hub, side);
      if (b.size) blobs.set(b.id, around(c, b.size));
      let y = c.y - stacked(b.disc.blobs) / 2;
      for (const kid of b.disc.blobs) {
        const s = whole(kid);
        if (!s) continue;
        settle(kid, { x: side > 0 ? c.x + r + RUN : c.x - r - RUN - s.w, y, ...s }, c.x);
        y += s.h + LANE_GAP;
      }
      return;
    }
    const size = b.size!;
    if (!b.lanes?.length) {
      blobs.set(b.id, room);
      return;
    }
    // The blob keeps its own size, half way up the room; the lanes stand one
    // under another beside it, in the order of their heads' rows, so the
    // lines fan out from the rows without crossing.
    const box: Rect = { x: side > 0 ? room.x : room.x + room.w - size.w, y: room.y + (room.h - size.h) / 2, ...size };
    blobs.set(b.id, box);
    sides.set(b.id, side);
    let y = room.y + (room.h - lanesHeight(b.lanes)) / 2;
    for (const lane of b.lanes) {
      const leaves = { x: side > 0 ? box.x + box.w : box.x, y: box.y + (lane.mid ?? size.h / 2) };
      hubs.set(lane.hub, leaves);
      const tall = stacked(lane.blobs);
      if (!tall) continue;
      let top = y;
      for (const kid of lane.blobs) {
        const s = whole(kid);
        if (!s) continue;
        settle(kid, { x: side > 0 ? box.x + box.w + RUN : box.x - RUN - s.w, y: top, ...s }, leaves.x);
        top += s.h + LANE_GAP;
      }
      y += tall + BETWEEN;
    }
  };

  for (const spec of specs) {
    const disc = { w: spec.r * 2, h: spec.r * 2 };
    let at: Point = { x: 0, y: 0 };
    let facing: number | null = null;
    if (spec.from) {
      const parent = hubs.get(spec.from.hub);
      const via = blobs.get(spec.from.blob);
      if (!parent || !via) continue;
      // The disc sits where the blob did, so nothing round it moves, and
      // takes over the room the blob held.
      at = center(via);
      facing = Math.atan2(at.y - parent.y, at.x - parent.x);
      const seat = seats.get(spec.from.blob);
      const i = seat ? placed.indexOf(seat) : -1;
      if (i >= 0) placed.splice(i, 1);
    }
    hubs.set(spec.id, at);
    placed.push(around(at, disc));

    // A blob against the hub at an angle, `out` further than snug.
    const seat = (size: Size, angle: number, out: number): Rect => {
      const d = spec.r + REACH + extent(size, angle) + out;
      return around({ x: at.x + Math.cos(angle) * d, y: at.y + Math.sin(angle) * d }, size);
    };
    // Each blob has its turn round the hub and takes the free seat nearest
    // it, moving out a little at a time when there is none. Within `room`, a
    // box centred on the hub, or not at all when it has no seat in it.
    const n = spec.blobs.length;
    const span = facing === null ? 2 * Math.PI : Math.min(FAN, EACH * n);
    const round = facing === null ? Math.PI : span / 2;
    const find = (i: number, size: Size, room: Size | null, mine: Rect[]): Rect | null => {
      const fits = (r: Rect) =>
        (!room ||
          (Math.abs(r.x + r.w / 2 - at.x) + r.w / 2 <= room.w / 2 &&
            Math.abs(r.y + r.h / 2 - at.y) + r.h / 2 <= room.h / 2)) &&
        free(r) &&
        free(r, mine);
      const far = room ? Math.hypot(room.w, room.h) / 2 : FAR;
      const turn = facing === null ? ROOT_START + (i * span) / n : facing - span / 2 + ((i + 0.5) * span) / n;
      const lo = facing === null ? -Infinity : facing - FAN / 2;
      const hi = facing === null ? Infinity : facing + FAN / 2;
      for (let out = 0; out <= far; out += STEP)
        for (let off = 0; off <= round; off += TURN)
          for (const angle of off === 0 ? [turn] : [turn + off, turn - off]) {
            if (angle < lo - 1e-9 || angle > hi + 1e-9) continue;
            const r = seat(size, angle, out);
            if (fits(r)) return r;
          }
      if (room) return null;
      for (let t = spec.r + REACH + extent(size, turn); ; t += STEP) {
        const r = around({ x: at.x + Math.cos(turn) * t, y: at.y + Math.sin(turn) * t }, size);
        if (fits(r)) return r;
      }
    };
    const sizes = spec.blobs.map(whole);
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
    const was = before?.hubs.get(spec.id);
    if (was && was.x === at.x && was.y === at.y) {
      // The hub is where it was: its blobs that have not changed size keep
      // their seats, and one that has (or is new) finds a seat round them.
      // Only the blob the reader opened moves.
      const mine: Rect[] = [];
      found = spec.blobs.map((blob, i) => {
        const size = sizes[i];
        const old = before!.seats.get(blob.id);
        const keeps = size && old && old.w === size.w && old.h === size.h && free(old) && free(old, mine);
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
      // The root's blobs are packed into the shape of the screen, in the
      // least room that takes them all, so the first view fits it as large
      // as can be.
      if (facing === null && frame)
        for (let scale = 1; !found && scale <= 3; scale += 0.06)
          found = ring({ w: frame.w * scale, h: frame.h * scale });
      found ??= ring(null)!;
    }
    spec.blobs.forEach((blob, i) => {
      const r = found[i];
      if (!r) return;
      seats.set(blob.id, r);
      placed.push(r);
      settle(blob, r, at.x);
    });
  }
  return { hubs, blobs, seats, sides };
}

// The box that holds a hub and its blobs, for fitting them on screen.
export function bounds(at: Layout, hub: string, r: number, blobIds: string[]): Rect | null {
  const c = at.hubs.get(hub);
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

// What a bowed line joins: a hub's disc or a blob's box.
export type Body = { c: Point; r: number } | Rect;

const isDisc = (b: Body): b is { c: Point; r: number } => "r" in b;
const middle = (b: Body): Point => (isDisc(b) ? b.c : center(b));

// Where a line from a body's middle towards a point leaves it.
function rim(b: Body, toward: Point): Point {
  if (!isDisc(b)) return edgeToward(b, toward);
  const angle = Math.atan2(toward.y - b.c.y, toward.x - b.c.x);
  return { x: b.c.x + Math.cos(angle) * b.r, y: b.c.y + Math.sin(angle) * b.r };
}

const f = (n: number) => n.toFixed(1);

// A gently bowed line from one body's edge to another's: round a disc.
export function join(from: Body, to: Body): Link {
  const start = rim(from, middle(to));
  const end = rim(to, middle(from));
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const bow = Math.sin(2 * Math.atan2(dy, dx)) >= 0 ? 0.14 : -0.14;
  const cx = (start.x + end.x) / 2 - dy * bow;
  const cy = (start.y + end.y) / 2 + dx * bow;
  return { d: `M ${f(start.x)} ${f(start.y)} Q ${f(cx)} ${f(cy)} ${f(end.x)} ${f(end.y)}`, end };
}

// A line that leaves and arrives level: in a rack, from a head's row to the
// side of one of its blobs.
export function wire(from: Point, to: Point): Link {
  const half = (to.x - from.x) / 2;
  return {
    d: `M ${f(from.x)} ${f(from.y)} C ${f(from.x + half)} ${f(from.y)} ${f(to.x - half)} ${f(to.y)} ${f(to.x)} ${f(to.y)}`,
    end: to,
    start: from,
  };
}
