// The constellation's camera and the reader's hands on it: where the canvas
// origin sits in the stage and the zoom (`view`), the part of the stage the
// reader can see (`screen`), and dragging, wheeling, zooming and keyboard
// focus. Knows nothing about what is drawn; the component tells it when the
// reader takes over and what to do when the visible area changes.

import { batch, createSignal, onCleanup, onMount } from "solid-js";
import { visibleStage, type CameraViewport } from "./camera.ts";
import type { Point } from "./layout.ts";

export interface View {
  x: number;
  y: number;
  k: number;
}

export interface CanvasOptions {
  stage: () => HTMLElement;
  minZoom: number;
  maxZoom: number;
  // The reader moved or zoomed the camera: automatic fitting stops.
  onTakeOver(): void;
  // The camera started moving under the pointer: close word details, the
  // pinned ones too when `pinned`.
  closeDetails(pinned: boolean): void;
  // The visible part of the stage was measured, for the first time when
  // `first`.
  onResize(screen: CameraViewport, first: boolean): void;
}

// Inside the canvas but not part of it: controls the pointer must reach.
const UI = ".cst-ui";

export function createCanvas(options: CanvasOptions) {
  const { stage } = options;
  const [view, setView] = createSignal<View>({ x: 0, y: 0, k: 1 });
  const [screen, setScreen] = createSignal<CameraViewport>({ x: 0, y: 0, w: 0, h: 0, scale: 1 });
  const [gliding, setGliding] = createSignal(false);
  const [dragging, setDragging] = createSignal(false);

  let glideTimer: number | undefined;
  const glide = (next: View) => {
    setGliding(true);
    setView(next);
    window.clearTimeout(glideTimer);
    glideTimer = window.setTimeout(() => setGliding(false), 700);
  };
  const centreOn = (p: Point) => {
    const { k } = view();
    glide({ x: screen().x + screen().w / 2 - p.x * k, y: screen().y + screen().h / 2 - p.y * k, k });
  };
  // Zoom to `k` about a point of the stage, keeping that point still.
  const zoomAt = (cx: number, cy: number, k: number, smooth = false) => {
    options.onTakeOver();
    const v = view();
    const next = Math.min(options.maxZoom / screen().scale, Math.max(options.minZoom / screen().scale, k));
    const to = { x: cx - ((cx - v.x) / v.k) * next, y: cy - ((cy - v.y) / v.k) * next, k: next };
    if (smooth) glide(to);
    else setView(to);
  };
  const zoomBy = (factor: number) =>
    zoomAt(screen().x + screen().w / 2, screen().y + screen().h / 2, view().k * factor, true);

  // Dragging moves the canvas. The pointer is captured only once it has
  // moved, so a plain click still reaches the word under it.
  let drag: { id: number; x: number; y: number; vx: number; vy: number; moved: boolean } | null = null;
  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0 || (e.target as Element).closest(UI)) return;
    drag = { id: e.pointerId, x: e.clientX, y: e.clientY, vx: view().x, vy: view().y, moved: false };
  };
  const onPointerMove = (e: PointerEvent) => {
    if (!drag || e.pointerId !== drag.id) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (!drag.moved) {
      if (Math.hypot(dx, dy) < 5) return;
      drag.moved = true;
      options.onTakeOver();
      stage().setPointerCapture(e.pointerId);
      options.closeDetails(true);
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
    const el = stage();
    el.scrollLeft = 0;
    el.scrollTop = 0;
    const target = e.target as HTMLElement;
    if (!target.matches(":focus-visible") || target.closest(UI)) return;
    const r = target.getBoundingClientRect();
    const box = el.getBoundingClientRect();
    const visible = screen();
    const s = { left: box.left + visible.x, top: box.top + visible.y,
      right: box.left + visible.x + visible.w, bottom: box.top + visible.y + visible.h };
    const m = 56 / visible.scale;
    const dx = r.left < s.left + m ? s.left + m - r.left : r.right > s.right - m ? s.right - m - r.right : 0;
    const dy = r.top < s.top + m ? s.top + m - r.top : r.bottom > s.bottom - m ? s.bottom - m - r.bottom : 0;
    if (dx || dy) glide({ ...view(), x: view().x + dx, y: view().y + dy });
  };

  onMount(() => {
    const el = stage();
    // The stage takes what is left of the window below the word's title.
    const fill = () =>
      el.style.setProperty("--cst-top", `${Math.round(el.getBoundingClientRect().top + window.scrollY)}px`);
    fill();
    // Camera coordinates use the visible part of the stage (browser pinch
    // zoom crops it without resizing the stage); keep the view's centre
    // where it was when that part moves or resizes.
    const measure = () => {
      const was = screen();
      const r = el.getBoundingClientRect();
      const visual = window.visualViewport;
      const now = visibleStage({ x: r.x, y: r.y, w: r.width, h: r.height }, {
        x: visual?.offsetLeft ?? 0, y: visual?.offsetTop ?? 0,
        w: visual?.width ?? window.innerWidth, h: visual?.height ?? window.innerHeight,
        scale: visual?.scale ?? 1,
      });
      if (!now.w || !now.h) return;
      if (now.x === was.x && now.y === was.y && now.w === was.w && now.h === was.h && now.scale === was.scale) return;
      const first = !was.w;
      batch(() => {
        options.onResize(now, first);
        if (first) setView({ x: now.x + now.w / 2, y: now.y + now.h / 2, k: 1 / now.scale });
        else {
          const v = view();
          setView({ ...v, x: v.x + now.x - was.x + (now.w - was.w) / 2, y: v.y + now.y - was.y + (now.h - was.h) / 2 });
        }
        setScreen(now);
      });
    };
    // Not passive: the wheel moves the canvas, not the page.
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      options.onTakeOver();
      options.closeDetails(false);
      const v = view();
      if (e.ctrlKey || e.metaKey) {
        const r = el.getBoundingClientRect();
        zoomAt(e.clientX - r.left, e.clientY - r.top, v.k * Math.exp(-e.deltaY * 0.01));
      } else {
        const unit = e.deltaMode === 1 ? 32 : 1;
        setGliding(false);
        setView({ ...v, x: v.x - e.deltaX * unit, y: v.y - e.deltaY * unit });
      }
    };
    const sized = new ResizeObserver(measure);
    sized.observe(el);
    window.addEventListener("resize", fill);
    window.visualViewport?.addEventListener("resize", measure);
    window.visualViewport?.addEventListener("scroll", measure);
    window.addEventListener("scroll", measure);
    el.addEventListener("wheel", onWheel, { passive: false });
    el.addEventListener("click", dropDragClick, true);
    onCleanup(() => {
      sized.disconnect();
      window.removeEventListener("resize", fill);
      window.visualViewport?.removeEventListener("resize", measure);
      window.visualViewport?.removeEventListener("scroll", measure);
      window.removeEventListener("scroll", measure);
      el.removeEventListener("wheel", onWheel);
      el.removeEventListener("click", dropDragClick, true);
      window.clearTimeout(glideTimer);
    });
  });

  return {
    view, setView, screen, gliding, dragging,
    glide, centreOn, zoomAt, zoomBy,
    // For the stage element.
    handlers: { onPointerDown, onPointerMove, onPointerUp, onPointerCancel: onPointerUp, onFocusIn },
  };
}
