import { renderTags } from "../helpers";
import { listen } from "../invoke/invoke";
import { playerHandle } from "../player/player";

declare const waveform__: HTMLDivElement;
declare const wave_thumb__: HTMLDivElement;

declare const preview_label__: HTMLSpanElement;
declare const preview_tags__: HTMLDivElement;

declare const s_total__: HTMLSpanElement;

function createPreview() {
  waveform__.onclick = (e) => {
    const rect = waveform__.getBoundingClientRect();
    const x = e.clientX - rect.left;

    const prog = x / rect.width;
    playerHandle.seek(prog);
  };

  return {
    set position(pos: number) {
      const margin = 0.1;
      const p = Math.max(Math.min(1, pos), 0) * 100;

      waveform__.style.backgroundSize = `${p}% 100%, ${100 - p}% 100%`;
      wave_thumb__.style.background = `linear-gradient(to right,transparent ${p - margin}%,var(--text-primary) ${p}%,transparent ${p + margin}%)`;
    },
    set img(uri: string) {
      waveform__.style.maskImage = `url(${uri})`;
    },

    set label(label: string) {
      preview_label__.textContent = label;
    },

    set tags(tags: string[]) {
      renderTags(preview_tags__, tags);
    },
  };
}

export const PreviewHandler = createPreview();

listen("read_audio", (uri) => {
  PreviewHandler.img = uri;
});

listen("s_tick", (n) => (s_total__.textContent = n));
