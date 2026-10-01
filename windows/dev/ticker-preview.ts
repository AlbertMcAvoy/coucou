// Dev harness: the overview ticker frozen at several points of one step
// transition, side by side, so overlaps can be screenshotted. Not in the bundle.

import "../src/style.css";
import { Ticker } from "../src/views/ticker";
import { h } from "../src/views/dom";
import type { AgentTask } from "../src/core/state";

const STEPS = [
  "Exécute · git status --short",
  "Exécute · git reset -q --soft HEAD~1",
  "Exécute · cd /home/myke/.claude/projects",
];
const task = (stepIndex: number) =>
  ({ id: "integration_claude", name: "coucou-perso", steps: STEPS, stepIndex } as unknown as AgentTask);

for (const p of [0, 0.25, 0.5, 0.75, 0.999, -1]) {
  const t = new Ticker();
  t.sync(task(1));
  t.sync(task(2)); // queues one transition
  t.tick(1000);
  if (p >= 0) t.tick(1000 + p * 380);
  else { t.tick(1000 + 380); }
  const who = h("div", { class: "who" }, h("span", { class: "name", text: "coucou-perso" }), h("span", { class: "tool", text: "Claude Code" }));
  document.body.append(
    h("div", { class: "frame" },
      h("div", { class: "card-body" }, who, t.el),
      h("div", { class: "label", text: p >= 0 ? `p=${p}` : "rest" }),
    ),
  );
}
