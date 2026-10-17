// Single app-wide popover state, driven by delegated events on the trees.
//
// Desktop (fine pointer): hovering a row opens the popover after a short
// delay; it stays while the pointer is over the row or the popover. Clicking
// a label pins it. Touch (coarse pointer): tapping a label opens it as a
// bottom sheet. Esc, outside click or the close button dismiss a pinned one.

import { createSignal } from "solid-js";
import { TreeNode } from "./model";

export interface PopoverState {
  node: TreeNode;
  anchor: HTMLElement;
  pinned: boolean;
}

export const [popover, setPopover] = createSignal<PopoverState | null>(null);

const SHOW_DELAY = 150;
const HIDE_DELAY = 200;
let showTimer: number | undefined;
let hideTimer: number | undefined;

export const isFinePointer = () =>
  window.matchMedia("(hover: hover) and (pointer: fine)").matches;

export function hoverEnter(node: TreeNode, anchor: HTMLElement) {
  if (!isFinePointer()) return;
  const current = popover();
  if (current?.pinned) return;
  window.clearTimeout(hideTimer);
  window.clearTimeout(showTimer);
  if (current && current.node === node) return;
  showTimer = window.setTimeout(() => {
    setPopover({ node, anchor, pinned: false });
  }, SHOW_DELAY);
}

export function hoverLeave() {
  window.clearTimeout(showTimer);
  const current = popover();
  if (!current || current.pinned) return;
  window.clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => {
    if (!popover()?.pinned) setPopover(null);
  }, HIDE_DELAY);
}

// Pointer moved into the popover itself: keep it.
export function popoverEnter() {
  window.clearTimeout(hideTimer);
}

export function pin(node: TreeNode, anchor: HTMLElement) {
  window.clearTimeout(showTimer);
  window.clearTimeout(hideTimer);
  const current = popover();
  if (current?.pinned && current.node === node) {
    setPopover(null);
    return;
  }
  setPopover({ node, anchor, pinned: true });
}

export function close() {
  window.clearTimeout(showTimer);
  window.clearTimeout(hideTimer);
  setPopover(null);
}
