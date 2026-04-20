import { renderTags } from "./helpers";

declare const playBtn: HTMLButtonElement;
declare const waveform: HTMLImageElement;
declare const preview_label: HTMLSpanElement;
declare const preview_tags: HTMLDivElement;

let playing = false;
playBtn.onclick = () => {
  playing = !playing;
  playBtn.textContent = playing ? "⏸" : "▶";
};

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

listen("read_audio", (uri) => (PreviewHandler.img = uri));
