import { renderTags } from "./helpers";

declare const waveform: HTMLImageElement;
declare const preview_label: HTMLSpanElement;
declare const preview_tags: HTMLDivElement;

declare const s_total: HTMLSpanElement;

function createPreview() {
  return {
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

listen("s_tick", (n) => (s_total.textContent = n));
