// Where the constellation's hubs and blobs go. Hubs are placed in the order
// they were opened, each where the blob it was opened from sits and its own
// blobs round what is already there, so opening a word never rearranges what
// came before it.

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

export interface HubSpec {
  id: string;
  // A hub is a disc of this radius, unless it is `within` a blob.
  r: number;
  // The blob (of an earlier hub) this hub was opened from; none for the root.
  // `within`: the blob holds other heads too, so it stays and stands for the
  // hub, whose blobs go round it. Otherwise the blob held nothing else and is
  // no longer drawn: the hub's disc takes its seat.
  from: { hub: string; blob: string; within: boolean } | null;
  // In the order they go round; a blob not measured yet has no size and is
  // left out of the result.
  blobs: { id: string; size: Size | undefined }[];
}

export interface Layout {
  hubs: Map<string, Point>;
  blobs: Map<string, Rect>;
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

export const center = (r: Rect): Point => ({ x: r.x + r.w / 2, y: r.y + r.h / 2 });

// How far a box reaches from its centre in a direction.
const extent = (s: Size, angle: number) =>
  (Math.abs(Math.cos(angle)) * s.w) / 2 + (Math.abs(Math.sin(angle)) * s.h) / 2;

const around = (c: Point, s: Size): Rect => ({ x: c.x - s.w / 2, y: c.y - s.h / 2, ...s });

const apart = (a: Rect, b: Rect) =>
  a.x >= b.x + b.w + GAP || b.x >= a.x + a.w + GAP || a.y >= b.y + b.h + GAP || b.y >= a.y + a.h + GAP;

// `frame`: the room the root and its blobs have, centred on the root.
export function layout(specs: HubSpec[], frame?: Size): Layout {
  const hubs = new Map<string, Point>();
  const blobs = new Map<string, Rect>();
  const placed: Rect[] = [];
  const free = (r: Rect, others: Rect[] = placed) => others.every((p) => apart(r, p));

  for (const spec of specs) {
    // What the hub's blobs go round: its disc, or the blob it is within. A
    // later hub sits where the blob it was opened from does, so nothing round
    // that blob moves.
    let body: Size = { w: spec.r * 2, h: spec.r * 2 };
    let disc = true;
    let at: Point = { x: 0, y: 0 };
    let facing: number | null = null;
    if (spec.from) {
      const parent = hubs.get(spec.from.hub);
      const via = blobs.get(spec.from.blob);
      if (!parent || !via) continue;
      at = center(via);
      facing = Math.atan2(at.y - parent.y, at.x - parent.x);
      if (spec.from.within) {
        body = via;
        disc = false;
      } else {
        // The blob gives the room it took up to the disc.
        const i = placed.indexOf(via);
        if (i >= 0) placed.splice(i, 1);
      }
    }
    hubs.set(spec.id, at);
    if (disc) placed.push(around(at, body));

    // How far from the hub's centre a blob's near edge is kept, and a blob
    // against the hub at an angle, `out` further than that.
    const clear = (angle: number) => (disc ? spec.r : extent(body, angle)) + REACH;
    const seat = (size: Size, angle: number, out: number): Rect => {
      const d = clear(angle) + extent(size, angle) + out;
      return around({ x: at.x + Math.cos(angle) * d, y: at.y + Math.sin(angle) * d }, size);
    };
    // Each blob has its turn round the hub (the root's all the way round, a
    // later hub's in a fan ahead of it) and takes the free seat nearest it,
    // moving out a little at a time when there is none. Within `room`, a box
    // centred on the hub, or not at all when a blob has no seat in it.
    const n = spec.blobs.length;
    const span = facing === null ? 2 * Math.PI : Math.min(FAN, EACH * n);
    const reach = facing === null ? Math.PI : span / 2;
    const ring = (room: Size | null): (Rect | null)[] | null => {
      const mine: Rect[] = [];
      const fits = (r: Rect) =>
        (!room ||
          (Math.abs(r.x + r.w / 2 - at.x) + r.w / 2 <= room.w / 2 &&
            Math.abs(r.y + r.h / 2 - at.y) + r.h / 2 <= room.h / 2)) &&
        free(r) &&
        free(r, mine);
      const far = room ? Math.hypot(room.w, room.h) / 2 : FAR;
      const seats: (Rect | null)[] = [];
      for (const [i, blob] of spec.blobs.entries()) {
        const size = blob.size;
        if (!size) {
          seats.push(null);
          continue;
        }
        const turn = facing === null ? ROOT_START + (i * span) / n : facing - span / 2 + ((i + 0.5) * span) / n;
        const lo = facing === null ? -Infinity : facing - FAN / 2;
        const hi = facing === null ? Infinity : facing + FAN / 2;
        let found: Rect | null = null;
        for (let out = 0; !found && out <= far; out += STEP) {
          for (let off = 0; !found && off <= reach; off += TURN) {
            for (const angle of off === 0 ? [turn] : [turn + off, turn - off]) {
              if (angle < lo - 1e-9 || angle > hi + 1e-9) continue;
              const r = seat(size, angle, out);
              if (fits(r)) {
                found = r;
                break;
              }
            }
          }
        }
        if (!found && room) return null;
        if (!found) {
          for (let t = clear(turn) + extent(size, turn); ; t += STEP) {
            found = around({ x: at.x + Math.cos(turn) * t, y: at.y + Math.sin(turn) * t }, size);
            if (fits(found)) break;
          }
        }
        mine.push(found!);
        seats.push(found);
      }
      return seats;
    };
    // The root's blobs are packed into the shape of the screen, in the least
    // room that takes them all, so the first view fits it as large as can be.
    let seats: (Rect | null)[] | null = null;
    if (facing === null && frame)
      for (let scale = 1; !seats && scale <= 3; scale += 0.06)
        seats = ring({ w: frame.w * scale, h: frame.h * scale });
    seats ??= ring(null)!;
    spec.blobs.forEach((blob, i) => {
      const r = seats[i];
      if (!r) return;
      blobs.set(blob.id, r);
      placed.push(r);
    });
  }
  return { hubs, blobs };
}

// The box that holds a hub and its blobs, for fitting them on screen.
export function bounds(at: Layout, hub: string, r: number, blobIds: string[]): Rect | null {
  const c = at.hubs.get(hub);
  if (!c) return null;
  const rects = [around(c, { w: r * 2, h: r * 2 }), ...blobIds.map((id) => at.blobs.get(id)).filter((b) => !!b)];
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
}

// What a line joins: a hub's disc or a blob's box.
export type Body = { c: Point; r: number } | Rect;

const isDisc = (b: Body): b is { c: Point; r: number } => "r" in b;
const middle = (b: Body): Point => (isDisc(b) ? b.c : center(b));

// Where a line from a body's middle towards a point leaves it.
function rim(b: Body, toward: Point): Point {
  if (!isDisc(b)) return edgeToward(b, toward);
  const angle = Math.atan2(toward.y - b.c.y, toward.x - b.c.x);
  return { x: b.c.x + Math.cos(angle) * b.r, y: b.c.y + Math.sin(angle) * b.r };
}

// A gently bowed line from one body's edge to another's.
export function join(from: Body, to: Body): Link {
  const start = rim(from, middle(to));
  const end = rim(to, middle(from));
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const bow = Math.sin(2 * Math.atan2(dy, dx)) >= 0 ? 0.14 : -0.14;
  const cx = (start.x + end.x) / 2 - dy * bow;
  const cy = (start.y + end.y) / 2 + dx * bow;
  const f = (n: number) => n.toFixed(1);
  return { d: `M ${f(start.x)} ${f(start.y)} Q ${f(cx)} ${f(cy)} ${f(end.x)} ${f(end.y)}`, end };
}
