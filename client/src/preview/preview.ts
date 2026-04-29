import { getCurrentSample } from "../browse/browse";
import { basename, renderTags, setLiked, setLikedView } from "../helpers";
import { listen } from "../invoke/invoke";
import { playerHandle } from "../player/player";

declare const waveform__: HTMLDivElement;
declare const wave_thumb__: HTMLDivElement;

declare const preview_label__: HTMLSpanElement;
declare const preview_tags__: HTMLDivElement;
declare const preview_fav__: HTMLSpanElement;

declare const s_total__: HTMLSpanElement;

preview_fav__.onclick = () => {
  const [path, isfav] = getCurrentSample() ?? [PreviewHandler.path, PreviewHandler.fav];
  if (path !== PreviewHandler.path) {
    setLiked(PreviewHandler.path, !PreviewHandler.fav);
    PreviewHandler.fav = !PreviewHandler.fav;
  } else {
    setLiked(path, !isfav);
    PreviewHandler.fav = !isfav;
  }
};

function createPreview() {
  waveform__.onclick = (e) => {
    const rect = waveform__.getBoundingClientRect();
    const x = e.clientX - rect.left;

    const prog = x / rect.width;
    playerHandle.seek(prog);
  };

  let path = "";

  return {
    get path() {
      return path;
    },
    set path(p: string) {
      path = p;
    },
    get label() {
      return basename(path);
    },
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

    set fav(state: boolean) {
      setLikedView(state, preview_fav__);
    },
  };
}

export const PreviewHandler = createPreview();

listen("read_audio", (uri) => {
  PreviewHandler.img = uri;
});

listen("s_tick", (n) => (s_total__.textContent = n));

listen("set-fav", (payload) => {
  const fav = !!+payload.charAt(0);
  const label = basename(payload.slice(1));

  if (PreviewHandler.label === label) {
    PreviewHandler.fav = fav;
  }
});
