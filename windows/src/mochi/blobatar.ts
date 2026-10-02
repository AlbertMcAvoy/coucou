// Blobatar as the island's character — a prototype, chosen in Settings →
// General → Character. The face comes from github.com/Alain00/blobatar (MIT):
// a round body and two capsule eyes, idling in CSS, morphing between
// expressions, and following the cursor.
//
// It stands in for Mochi's drawing only. Mochi's engine keeps running behind it
// for the timing (states, emotes), and Mochi himself still plays the greeting
// and the drop sequence: blobatar has no hands to wave and no mouth to take a
// file with.
//
// `blobatar/internal` is the entry point blobatar's own framework adapters use;
// Coucou has no framework, so it builds the same markup they do. That entry is
// only stable within a major, which is why package.json pins the version.

import "blobatar/motion.css";
import "blobatar/gaze.css";
import { blobatar } from "blobatar/blob";
import * as X from "blobatar/expression";
import { gaze, type Gaze } from "blobatar/gaze";
import { _layout, _parts } from "blobatar/internal";
import type { BotEmoteName, BotStateName } from "../core/layout";

type Expression = typeof X.idle;
type Options = NonNullable<Parameters<typeof _parts>[1]>;

/** Without a string of the user's: one character for everyone, round and white like Mochi. */
const SEED = "Mochi";
const TRAITS = { shape: 0.05 };
const WHITE = "#f5f6f8";
const INK = "#14151a";

const FOR_STATE: Record<BotStateName, Expression> = {
  idle: X.idle,
  working: X.thinking,
  thinking: X.thinking,
  searching: X.thinking,
  approval: X.surprised,
  question: X.unsure,
  error: X.sad,
  finished: X.happy,
  ratelimit: X.sick,
  sleeping: X.sleepy,
  dizzy: X.scared,
};

const FOR_EMOTE: Record<BotEmoteName, Expression> = {
  love: X.love,
  surprised: X.surprised,
  proud: X.smug,
  wink: X.wink,
  yawn: X.sleepy,
  happy: X.happy,
  annoyed: X.mad,
};

/** What the face shows for Mochi's state, an emote winning over it. */
export function expressionFor(state: BotStateName, emote: BotEmoteName | null): Expression {
  return emote ? FOR_EMOTE[emote] : FOR_STATE[state];
}

/**
 * The name and options a face is drawn from. No `seed`: the classic round white
 * one. A seed: the creature that string makes — shape, colour, eyes, as in
 * blobatar's demo. Either way an integration's colour wins over the creature's
 * own: the body wears it exactly, so a pill and its character read as one.
 */
function look(seed: string, expression: Expression, color: string | null): { name: string; opts: Options } {
  const name = seed.trim() || SEED;
  const traits = seed.trim() ? undefined : TRAITS;
  if (color) return { name, opts: { expression, traits, palette: { head: color, eye: inkOn(color) } } };
  return { name, opts: seed.trim() ? { expression } : { expression, traits, palette: { head: WHITE, eye: INK } } };
}

/**
 * Eyes that read on `hex`: dark ink, or light on a dark body, whichever has more
 * contrast. A palette passed to blobatar skips its own contrast guarantee, so
 * this is that guarantee's stand-in.
 */
function inkOn(hex: string): string {
  const n = parseInt(hex.slice(1, 7), 16);
  const lin = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((c) => lin(c / 255));
  const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  const contrast = (a: number, b: number) => (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
  // Relative luminances of INK and WHITE.
  return contrast(lum, 0.0076) >= contrast(lum, 0.921) ? INK : WHITE;
}

/**
 * Where the body sits in the 100-unit viewBox: its centre, and how much of the
 * box its widest axis spans. Every silhouette has its own, so the face is sized
 * and centred from it and stays the size and place of Mochi whatever its shape.
 */
function bodyFrame(name: string, opts: Options) {
  const body = _layout(name, opts).body;
  return { cx: body.cx / 100, cy: body.cy / 100, span: (2 * Math.max(body.rx, body.ry)) / 100 };
}

const NS = "http://www.w3.org/2000/svg";

/** The island's blobatar: one inline SVG, animated, kept for the app's life. */
export class BlobatarFace {
  readonly el: SVGSVGElement;
  private body: SVGGElement;
  private expression: Expression | null = null;
  private color: string | null | undefined = undefined;
  private seed: string | undefined = undefined;
  private frame = { cx: 0.5, cy: 0.5, span: 0.68 };
  private vars: string[] = [];
  private eyes: Gaze | null = null;

  constructor() {
    this.el = document.createElementNS(NS, "svg");
    this.el.setAttribute("viewBox", "0 0 100 100");
    this.el.setAttribute("aria-hidden", "true");
    this.el.classList.add("blobatar-face");
    // How far the eyes may follow the cursor, in viewBox units: gaze.css
    // registers it at 0, so without it the eyes never move.
    this.el.style.setProperty("--mo-track-travel", "3px");
    this.body = document.createElementNS(NS, "g");
    this.el.append(this.body);
  }

  /**
   * Shows the creature `seed` makes (empty: the classic one) wearing
   * `expression`, in `color` (null: its own). A new expression only swaps the
   * root's class and the pose variables, so the CSS transition morphs the face;
   * a new seed or colour redraws it.
   */
  show(seed: string, expression: Expression, color: string | null) {
    if (seed === this.seed && expression === this.expression && color === this.color) return;
    const { name, opts } = look(seed, expression, color);
    const parts = _parts(name, { animate: "always", ...opts });
    if (seed !== this.seed || color !== this.color) {
      this.body.innerHTML = parts.inner;
      this.frame = bodyFrame(name, opts);
    }
    this.body.setAttribute("class", parts.cls ?? "");
    for (const key of this.vars) this.el.style.removeProperty(key);
    this.vars = Object.keys(parts.vars ?? {});
    for (const [key, value] of Object.entries(parts.vars ?? {})) this.el.style.setProperty(key, value);
    // The tinted poses (sick, mad, love, shy) warm or green the body; an
    // integration's colour stays as it is, and the eyes still tell them apart.
    if (color) this.el.style.setProperty("--mo-head", color);
    this.seed = seed;
    this.expression = expression;
    this.color = color;
  }

  /** Centres the body on (cx, cy), `diameter` across, in island pixels. */
  place(cx: number, cy: number, diameter: number) {
    const size = diameter / this.frame.span;
    this.el.style.width = `${size}px`;
    this.el.style.height = `${size}px`;
    this.el.style.left = `${cx - size * this.frame.cx}px`;
    this.el.style.top = `${cy - size * this.frame.cy}px`;
  }

  /**
   * Looks the way Mochi would: `lookX` / `lookY` are his -1…1 leanings toward
   * the cursor, turned into a point that far off the face's centre.
   */
  look(lookX: number, lookY: number) {
    this.eyes ??= gaze(this.el);
    const r = this.el.getBoundingClientRect();
    if (r.width === 0) return;
    this.eyes.lookAt({ x: r.left + r.width / 2 + lookX * 200, y: r.top + r.height / 2 - lookY * 200 });
  }

  /** Hidden: no box, so its CSS animations stop and cost nothing. */
  setVisible(on: boolean) {
    this.el.style.display = on ? "" : "none";
  }

  /** Still: the idle loops hold, as Mochi does when the island rests. */
  setStill(on: boolean) {
    this.el.classList.toggle("blobatar-still", on);
  }
}

/**
 * A pill's blobatar: static markup, as blobatar recommends for a row of them,
 * and how much of its box the body spans — the box is sized from that.
 */
export function miniBlobatar(
  seed: string,
  expression: Expression,
  color: string | null,
): { markup: string; span: number } {
  const { name, opts } = look(seed, expression, color);
  let markup = blobatar(name, opts);
  // As on the island: a tinted pose doesn't get to recolour an integration.
  // The static markup bakes the tint in, so it is swapped back.
  const tinted = color ? _parts(name, { animate: "always", ...opts }).vars?.["--mo-head"] : undefined;
  if (color && tinted && tinted.toLowerCase() !== color.toLowerCase()) markup = markup.split(tinted).join(color);
  return { markup, span: bodyFrame(name, opts).span };
}
