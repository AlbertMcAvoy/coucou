// Dev harness: the blobatar face for each of Mochi's states — the classic one,
// then the creature a string makes, alone and in an integration's hue — and a
// row of creatures from different strings at pill size. Not part of the app
// bundle.

import "../src/style.css";
import { BlobatarFace, expressionFor, miniBlobatar } from "../src/mochi/blobatar";
import type { BotStateName } from "../src/core/layout";

const STATES: BotStateName[] = [
  "idle", "working", "approval", "question", "error", "finished", "ratelimit", "sleeping", "dizzy",
];

for (const [seed, color] of [["", null], ["", "#FC6D26"], ["alain@example.com", null], ["alain@example.com", "#FF318C"]] as const) {
  const row = document.createElement("div");
  row.className = "row";
  for (const state of STATES) {
    const cell = document.createElement("div");
    cell.className = "cell";
    const face = new BlobatarFace();
    cell.append(face.el);
    face.show(seed, expressionFor(state, null), color);
    // The island's open Mochi is 58 px across.
    face.place(60, 54, 58);
    const label = document.createElement("span");
    label.textContent = state;
    cell.append(label);
    row.append(cell);
  }
  document.body.append(row);
}

const minis = document.createElement("div");
minis.className = "row";
for (const seed of ["", "Mochi", "mcg", "Coucou", "louis", "🦊", "sun-test-42", "alain@example.com"]) {
  const slot = document.createElement("span");
  slot.className = "mini";
  slot.title = seed || "(classic)";
  slot.style.width = slot.style.height = "40px";
  const { markup, span } = miniBlobatar(seed, expressionFor("idle", null), null);
  slot.innerHTML = markup;
  const svg = slot.firstElementChild as SVGSVGElement;
  svg.style.width = svg.style.height = `${40 / span}px`;
  minis.append(slot);
}
document.body.append(minis);
