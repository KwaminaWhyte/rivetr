import type { Edge, Node } from "@xyflow/react";
import { SHARED_ENV_ID, type CanvasProject } from "./use-canvas-data";

const CARD_W = 248;
const CARD_H = 104;
const GAP = 14;
const ENV_PAD = 14;
const ENV_HEAD = 32;
const ENV_EMPTY_H = 76;
const ENV_GAP = 18;
const PROJ_PAD = 18;
const PROJ_HEAD = 64;
const PROJ_GAP = 48;
const ROW_MAX_W = 2000;
const PROJ_MIN_W = 320;

/**
 * Deterministic grid layout: projects flow left to right in rows, each
 * project lays its environments side by side, and each environment stacks
 * its resource cards in up to two columns. Child positions are relative to
 * their parent (React Flow sub flows).
 */
export function layoutCanvas(
  projects: CanvasProject[],
  selectedId: string | null
): { nodes: Node[]; ids: Set<string> } {
  const nodes: Node[] = [];
  const ids = new Set<string>();
  let x = 0;
  let y = 0;
  let rowH = 0;

  for (const project of projects) {
    const envBoxes = project.environments.map((env) => {
      const n = env.resources.length;
      const cols = n > 2 ? 2 : Math.max(1, n);
      const rows = Math.max(1, Math.ceil(n / cols));
      const w = cols * CARD_W + (cols - 1) * GAP + ENV_PAD * 2;
      const h = n === 0 ? ENV_EMPTY_H : ENV_HEAD + rows * CARD_H + (rows - 1) * GAP + ENV_PAD;
      return { env, cols, w, h };
    });

    const innerW =
      envBoxes.reduce((sum, b) => sum + b.w, 0) + Math.max(0, envBoxes.length - 1) * ENV_GAP;
    const projW = Math.max(PROJ_MIN_W, innerW + PROJ_PAD * 2);
    const projH = PROJ_HEAD + Math.max(ENV_EMPTY_H, ...envBoxes.map((b) => b.h)) + PROJ_PAD;

    if (x > 0 && x + projW > ROW_MAX_W) {
      x = 0;
      y += rowH + PROJ_GAP;
      rowH = 0;
    }

    const projectNodeId = `p:${project.id}`;
    nodes.push({
      id: projectNodeId,
      type: "project",
      position: { x, y },
      data: { project },
      style: { width: projW, height: projH },
      draggable: false,
      selectable: false,
      zIndex: 0,
    });

    let ex = PROJ_PAD;
    for (const box of envBoxes) {
      const envNodeId = `e:${project.id}:${box.env.id}`;
      nodes.push({
        id: envNodeId,
        type: "environment",
        parentId: projectNodeId,
        position: { x: ex, y: PROJ_HEAD },
        data: {
          name: box.env.name,
          isDefault: box.env.isDefault,
          empty: box.env.resources.length === 0,
          shared: box.env.id === SHARED_ENV_ID,
        },
        style: { width: box.w, height: box.h },
        draggable: false,
        selectable: false,
        zIndex: 1,
      });
      box.env.resources.forEach((r, i) => {
        const col = i % box.cols;
        const row = Math.floor(i / box.cols);
        nodes.push({
          id: r.id,
          type: "resource",
          parentId: envNodeId,
          position: {
            x: ENV_PAD + col * (CARD_W + GAP),
            y: ENV_HEAD + row * (CARD_H + GAP),
          },
          data: { resource: r, selected: r.id === selectedId },
          style: { width: CARD_W, height: CARD_H },
          draggable: false,
          zIndex: 2,
        });
        ids.add(r.id);
      });
      ex += box.w + ENV_GAP;
    }

    x += projW + PROJ_GAP;
    rowH = Math.max(rowH, projH);
  }

  return { nodes, ids };
}

export function layoutEdges(
  links: { from: string; to: string; label: string }[],
  visible: Set<string>
): Edge[] {
  return links
    .filter((l) => visible.has(l.from) && visible.has(l.to))
    .map((l) => ({
      id: `${l.from}->${l.to}`,
      source: l.from,
      target: l.to,
      animated: true,
      zIndex: 3,
    }));
}
