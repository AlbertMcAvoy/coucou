// Mini Mochis (pills + compact grid) — port of MiniBotCanvasView.
// Each canvas owns a BotEngine; the island's frame loop ticks every live one.

import { BotEngine, hexToRGB } from "./engine";
import { expressionFor, miniBlobatar } from "./blobatar";
import { State, type AgentTask } from "../core/state";

interface MiniBot {
  canvas: HTMLCanvasElement;
  engine: BotEngine;
  cssSize: number;
  taskId: string;
}

const live = new Map<HTMLCanvasElement, MiniBot>();

/**
 * Pills with blobatar as the character (Settings → General → Character): static
 * markup, redrawn when the pill's state or colour changes, and no frames at all.
 */
interface MiniBlob {
  slot: HTMLElement;
  taskId: string;
  bodySize: number;
  drawn: string;
}

const blobs = new Map<HTMLElement, MiniBlob>();

function drawBlob(mb: MiniBlob, task: AgentTask) {
  const seed = State.settings.characterSeed ?? "";
  const key = `${seed}|${task.state}|${task.emote ?? ""}|${task.color}`;
  if (key === mb.drawn) return;
  mb.drawn = key;
  const { markup, span } = miniBlobatar(seed, expressionFor(task.state, task.emote ?? null), task.color);
  mb.slot.innerHTML = markup;
  const svg = mb.slot.firstElementChild as SVGSVGElement | null;
  if (svg) {
    const size = mb.bodySize / span;
    svg.style.width = `${size}px`;
    svg.style.height = `${size}px`;
  }
}

/**
 * Creates a mini Mochi whose **body** is `bodySize` CSS pixels across.
 *
 * The engine draws the body at 60 % of its canvas, so the canvas is
 * `bodySize / 0.6` and is centred in a `bodySize` slot, overflowing it — the
 * same thing SwiftUI does with a `.frame(width: 22/0.6)` inside a
 * `.frame(width: 22)`. Sizing the canvas itself to `bodySize` would shrink the
 * whole drawing to 60 %, which is what used to happen.
 */
export function createMiniBot(task: AgentTask, bodySize: number): HTMLElement {
  const slot = document.createElement("span");
  slot.className = "mini";
  slot.style.width = `${bodySize}px`;
  slot.style.height = `${bodySize}px`;

  if (State.settings.character === "blobatar") {
    const mb: MiniBlob = { slot, taskId: task.id, bodySize, drawn: "" };
    drawBlob(mb, task);
    blobs.set(slot, mb);
    return slot;
  }

  const canvas = document.createElement("canvas");
  const engineSize = bodySize / 0.6;
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  canvas.width = Math.round(engineSize * dpr);
  canvas.height = Math.round(engineSize * dpr);
  canvas.style.width = `${engineSize}px`;
  canvas.style.height = `${engineSize}px`;
  slot.append(canvas);

  const engine = new BotEngine();
  engine.isMini = true;
  engine.bodyColor = hexToRGB(task.color);
  engine.setState(task.state, true);
  if (task.emote) engine.setPermanentEmote(task.emote);
  if (task.miniEye) {
    engine.permanentEye = task.miniEye;
    engine.eyeOverride = task.miniEye;
    engine.eyeOverrideUntil = Number.POSITIVE_INFINITY;
  }

  live.set(canvas, { canvas, engine, cssSize: engineSize, taskId: task.id });
  return slot;
}

export function releaseMiniBot(canvas: HTMLCanvasElement) {
  live.delete(canvas);
}

/** Drops every canvas no longer in the document (views are rebuilt wholesale). */
export function pruneMiniBots() {
  for (const [canvas] of live) {
    if (!canvas.isConnected) live.delete(canvas);
  }
  for (const [slot] of blobs) {
    if (!slot.isConnected) blobs.delete(slot);
  }
}

export function syncMiniBotStates(tasks: AgentTask[]) {
  for (const mb of blobs.values()) {
    const task = tasks.find((t) => t.id === mb.taskId);
    if (task) drawBlob(mb, task);
  }
  for (const mb of live.values()) {
    const task = tasks.find((t) => t.id === mb.taskId);
    if (!task) continue;
    mb.engine.setState(task.state);
    mb.engine.bodyColor = hexToRGB(task.color);
  }
}

export function tickMiniBots(dt: number) {
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  for (const mb of live.values()) {
    const ctx = mb.canvas.getContext("2d");
    if (!ctx) continue;
    mb.engine.update(dt);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, mb.cssSize, mb.cssSize);
    mb.engine.draw(ctx, mb.cssSize, mb.cssSize);
  }
}

export const miniBotCount = () => live.size;
