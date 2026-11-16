// The constellation's cables: which connections a drawing has and where they
// run, from the placed layout and the open hubs. Pure; the component passes
// in what it knows about each blob (`CableScene`) and draws the result.

import type { Group } from "./branches.ts";
import { blobId, type DisplayBranch, type Hub } from "./hubs.ts";
import {
  STEM_GUTTER,
  cableRouter,
  join,
  rootDirection,
  sharedWires,
  stemArrival,
  type Layout,
  type Link,
  type Point,
  type Rect,
} from "./layout.ts";

export interface Cable {
  hue: string;
  d: string;
  // Revealed by the latest click, and accented for a moment.
  fresh: boolean;
}

// One opened row, or the remaining preview (no head), of a blob.
export interface Entry {
  id: string;
  head: Hub | undefined;
}

export interface CableScene {
  hubs: Hub[];
  at: Layout;
  rootRadius: number;
  // Hubs revealed by the latest click.
  recent: ReadonlySet<string>;
  // Where a blob is drawn: its seat, or the part of it a shrunken blob uses.
  drawn(hub: Hub, blob: string): Rect | undefined;
  // A blob's remaining preview and opened rows, in drawing order.
  entries(hub: Hub, branch: DisplayBranch): Entry[];
  // Whether a blob is drawn as a column of separate source cards.
  clustered(hub: Hub, branch: DisplayBranch): boolean;
  // The groups of a blob's preview rows that are not opened yet.
  remainingGroups(hub: Hub, branch: DisplayBranch): Group[];
}

export function cables(scene: CableScene): Cable[] {
  const { hubs, at, recent, drawn } = scene;
  const sourcesOf = (h: Hub, b: DisplayBranch) => scene.entries(h, b).flatMap((entry) => at.sources.get(entry.id) ?? []);
  const cards = hubs.flatMap((h) => h.branches.flatMap((b) =>
    scene.clustered(h, b) ? sourcesOf(h, b) : drawn(h, blobId(h, b)) ?? []));
  // Stems the root's opened columns run along, kept clear by other cables.
  const stems = hubs.flatMap((h) => h.branches.flatMap((b) => {
    const id = blobId(h, b), box = drawn(h, id);
    const sources = sourcesOf(h, b);
    if (h.via || !box || sources.length < 2) return [];
    const side = at.sides.get(id) ?? 1;
    return [{ x: side > 0 ? box.x - STEM_GUTTER : box.x + box.w + STEM_GUTTER,
      top: Math.min(...sources.map((s) => s.y + s.h / 2)), bottom: Math.max(...sources.map((s) => s.y + s.h / 2)) }];
  }));
  const route = cableRouter(cards, stems);
  const out: Cable[] = [];
  const push = (hue: string, l: Link, fresh = false) => out.push({ hue, d: l.d, fresh });
  // Group incoming edges before routing. Shared previews have one gutter
  // connecting all of their parents, regardless of expansion order.
  const incoming = new Map<string, { from: Point[]; to: Point; hue: string; fresh: boolean }>();
  const connect = (id: string, from: Point, to: Point, hue: string, fresh: boolean) => {
    const group = incoming.get(id);
    if (group) { group.from.push(from); group.fresh ||= fresh; }
    else incoming.set(id, { from: [from], to, hue, fresh });
  };
  const otherSeats = (id: string) => [...at.seats].filter(([seat]) => seat !== id).map(([, seat]) => seat);
  for (const h of hubs) {
    const c = at.hubs.get(h.id);
    if (!c) continue;
    // From the root's rim to each blob round it; from a head's row to the
    // near side of each blob in its lane.
    const side = h.via ? (at.sides.get(h.via) ?? 1) : 0;
    for (const shared of h.shared) {
      const id = blobId(shared.host, shared.branch);
      const box = drawn(shared.host, id);
      if (sourcesOf(shared.host, shared.branch).length) continue; // Expanded entries get their own recorded edges below.
      if (box && side) connect(id, c, { x: side > 0 ? box.x : box.x + box.w, y: box.y + box.h / 2 }, h.hue!, recent.has(h.id));
    }
    for (const b of h.branches) {
      const id = blobId(h, b);
      const box = drawn(h, id);
      if (!box) continue;
      const hue = h.hues.get(b.key)!;
      const sources = sourcesOf(h, b);
      const outward = at.sides.get(id) ?? (side || (box.x + box.w / 2 >= c.x ? 1 : -1));
      const nearEdge = (r: Rect): Point => ({ x: outward > 0 ? r.x : r.x + r.w, y: r.y + r.h / 2 });
      if (!side && sources.length === 1) {
        // A single root card needs one direct connection, not a shared
        // stem approached above the card and then doubled back down.
        push(hue, join(c, scene.rootRadius, { ...nearEdge(sources[0]), w: 0, h: 0 }, otherSeats(id),
          rootDirection(h.branches.indexOf(b), h.branches.length)));
        continue;
      }
      if (side && sources.length) {
        // A head's opened column: each card from the hubs that recorded it.
        for (const entry of scene.entries(h, b)) {
          const card = at.sources.get(entry.id);
          if (!card) continue;
          const groups = entry.head ? b.groups.filter((g) => g.head.item.id === entry.head!.node.item.id)
            : scene.remainingGroups(h, b);
          const parentIds = new Set(groups.flatMap((g) => [...(b.parents?.get(g.head.item.id) ?? [h.id])]));
          for (const parent of parentIds) {
            const from = at.hubs.get(parent);
            if (from) connect(entry.id, from, nearEdge(card), hue, recent.has(parent));
          }
        }
        continue;
      }
      // Any sources left here are the root's opened column.
      const spineX = outward > 0 ? box.x - STEM_GUTTER : box.x + box.w + STEM_GUTTER;
      const arrival = sources.length ? { ...stemArrival(c, spineX, sources), w: 0, h: 0 } : box;
      if (side) connect(id, c, { x: side > 0 ? arrival.x : arrival.x + arrival.w, y: arrival.y + arrival.h / 2 }, hue, recent.has(h.id));
      else push(hue, join(c, scene.rootRadius, arrival, otherSeats(id),
        rootDirection(h.branches.indexOf(b), h.branches.length), sources.length > 0));
      if (sources.length) {
        const ys = sources.map((s) => s.y + s.h / 2);
        const top = Math.min(...ys, arrival.y), bottom = Math.max(...ys, arrival.y);
        // A zero-length SVG stroke with round caps renders as a dot.
        if (top !== bottom) push(hue, { d: `M ${spineX} ${top} L ${spineX} ${bottom}`, end: { x: spineX, y: arrival.y } });
        for (const source of sources) {
          const y = source.y + source.h / 2;
          const fresh = hubs.some((k) => recent.has(k.id) && at.sources.get(k.id) === source);
          push(hue, route({ x: spineX, y }, { x: nearEdge(source).x, y }), fresh);
        }
      }
    }
  }
  let sharedLane = 0;
  for (const group of incoming.values()) {
    const paths = group.from.length > 1 ? sharedWires(group.from, group.to, cards, sharedLane++) : [route(group.from[0], group.to)];
    for (const path of paths) push(group.hue, path, group.fresh);
  }
  return out;
}
