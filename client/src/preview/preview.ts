import { renderTags } from "../helpers";

declare const waveform: HTMLDivElement;
declare const wave_thumb: HTMLDivElement;

declare const preview_label: HTMLSpanElement;
declare const preview_tags: HTMLDivElement;

declare const s_total: HTMLSpanElement;

function createPreview() {
  return {
    set position(pos: number) {
      const margin = 0.1;
      const p = Math.max(Math.min(1, pos), 0) * 100;

      waveform.style.backgroundSize = `${p}% 100%, ${100 - p}% 100%`;
      wave_thumb.style.background = `linear-gradient(to right,transparent ${p - margin}%,var(--text-primary) ${p}%,transparent ${p + margin}%)`;
    },
    set img(uri: string) {
      waveform.style.maskImage = `url(${uri})`;
    },

    set label(label: string) {
      preview_label.textContent = label;
    },

    set tags(tags: string[]) {
      renderTags(preview_tags, tags);
    },
  };
}

export const PreviewHandler = createPreview();

listen("read_audio", (uri) => {
  PreviewHandler.img = uri;
});

PreviewHandler.img = "athumb://_/RFQDzvStdAQ";
PreviewHandler.position = 0.8;

listen("s_tick", (n) => (s_total.textContent = n));
