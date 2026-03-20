// This file has absolutely nothing to do with the Dungeons & Dragons role-playing game.
import type { Action } from "svelte/action";

export const SPIRE_PARTICLE_MIME = "application/x-spire-particle";

export interface PdgDragPayload {
  type: "pdg-particle";
  pdgId: number;
  label?: string;
  edition?: string;
  sourceId?: string;
}

function serialize(payload: PdgDragPayload): string {
  return JSON.stringify(payload);
}

export function parseParticlePayload(raw: string | null | undefined): PdgDragPayload | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<PdgDragPayload>;
    if (parsed.type !== "pdg-particle" || typeof parsed.pdgId !== "number") {
      return null;
    }
    return {
      type: "pdg-particle",
      pdgId: parsed.pdgId,
      label: typeof parsed.label === "string" ? parsed.label : undefined,
      edition: typeof parsed.edition === "string" ? parsed.edition : undefined,
      sourceId: typeof parsed.sourceId === "string" ? parsed.sourceId : undefined,
    };
  } catch {
    return null;
  }
}

export const draggableParticle: Action<HTMLElement, PdgDragPayload> = (node, initialPayload) => {
  let payload = initialPayload;

  const handleDragStart = (event: DragEvent): void => {
    if (!event.dataTransfer || !payload) return;
    const data = serialize(payload);
    event.dataTransfer.setData(SPIRE_PARTICLE_MIME, data);
    event.dataTransfer.setData("text/plain", payload.label ?? String(payload.pdgId));
    event.dataTransfer.effectAllowed = "copy";
    node.setAttribute("data-dragging", "true");
  };

  const handleDragEnd = (): void => {
    node.removeAttribute("data-dragging");
  };

  node.setAttribute("draggable", "true");
  node.addEventListener("dragstart", handleDragStart);
  node.addEventListener("dragend", handleDragEnd);

  return {
    update(nextPayload) {
      payload = nextPayload;
    },
    destroy() {
      node.removeEventListener("dragstart", handleDragStart);
      node.removeEventListener("dragend", handleDragEnd);
      node.removeAttribute("draggable");
      node.removeAttribute("data-dragging");
    },
  };
};

export interface ParticleDropzoneOptions {
  onDrop: (payload: PdgDragPayload) => void | Promise<void>;
  disabled?: boolean;
}

export const particleDropzone: Action<HTMLElement, ParticleDropzoneOptions> = (node, initialOptions) => {
  let options = initialOptions;

  const setHover = (value: boolean): void => {
    if (value) {
      node.setAttribute("data-drop-hover", "true");
      node.classList.add("drop-hover");
    } else {
      node.removeAttribute("data-drop-hover");
      node.classList.remove("drop-hover");
    }
  };

  const readPayload = (event: DragEvent): PdgDragPayload | null => {
    const transfer = event.dataTransfer;
    if (!transfer) return null;
    const data = transfer.getData(SPIRE_PARTICLE_MIME);
    return parseParticlePayload(data);
  };

  const handleDragOver = (event: DragEvent): void => {
    if (options.disabled) return;
    if (!readPayload(event)) return;
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "copy";
    }
    setHover(true);
  };

  const handleDragEnter = (event: DragEvent): void => {
    if (options.disabled) return;
    if (!readPayload(event)) return;
    event.preventDefault();
    setHover(true);
  };

  const handleDragLeave = (): void => {
    setHover(false);
  };

  const handleDrop = async (event: DragEvent): Promise<void> => {
    setHover(false);
    if (options.disabled) return;
    const payload = readPayload(event);
    if (!payload) return;
    event.preventDefault();
    await options.onDrop(payload);
  };

  node.addEventListener("dragover", handleDragOver);
  node.addEventListener("dragenter", handleDragEnter);
  node.addEventListener("dragleave", handleDragLeave);
  node.addEventListener("drop", handleDrop);

  return {
    update(nextOptions) {
      options = nextOptions;
    },
    destroy() {
      node.removeEventListener("dragover", handleDragOver);
      node.removeEventListener("dragenter", handleDragEnter);
      node.removeEventListener("dragleave", handleDragLeave);
      node.removeEventListener("drop", handleDrop);
      setHover(false);
    },
  };
};
