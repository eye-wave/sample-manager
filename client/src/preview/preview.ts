import { toggleFav } from "../api";
import { renderTags, setLikedView } from "../helpers";
import { missingUri } from "./missing";
import { onFavChanged, onWaveformReady } from "@state";
import type { Readable, Writable } from "../store";

type PreviewStores = {
  currentPath: Writable<string | null>;
  position: Readable<number>;
  previewMeta: Writable<{ label: string; tags: string[]; fav: boolean }>;
  waveformUri: Writable<string | null>;
};

export const createPreview = (stores: PreviewStores, seek: (pos: number) => void) => {
  // --- seek ---
  waveform__.onclick = (e) => {
    const rect = waveform__.getBoundingClientRect();
    seek((e.clientX - rect.left) / rect.width);
  };

  // --- position ---
  stores.position.subscribe((pos) => {
    const p = Math.max(Math.min(1, pos), 0) * 100;
    const margin = 0.1;
    waveform__.style.backgroundSize = `${p}% 100%, ${100 - p}% 100%`;
    wave_thumb__.style.background = `linear-gradient(to right,transparent ${p - margin}%,var(--text-primary) ${p}%,transparent ${p + margin}%)`;
  });

  // --- waveform image ---
  let currentUri = "";

  onWaveformReady.on((uri) => stores.waveformUri.set(uri));

  stores.waveformUri.subscribe((uri) => {
    if (!uri || currentUri === uri) return;
    waveform__.style.maskImage = `url("${
      uri === "ff-missing"
        ? missingUri(
            "FFMPEG couldn't be found on your system path, install it or set it manually in the app settings.",
          )
        : uri
    }")`;
    currentUri = uri;
  });

  // --- meta ---
  stores.previewMeta.subscribe(({ label, tags, fav }) => {
    preview_label__.textContent = label;
    renderTags(preview_tags__, tags);
    setLikedView(fav, preview_fav__);
  });

  // --- fav ---
  preview_fav__.onclick = () => {
    const path = stores.currentPath.get();
    if (!path) return;
    toggleFav(path);
  };

  onFavChanged.on(({ path, fav }) => {
    if (stores.currentPath.get() === path) stores.previewMeta.update((m) => ({ ...m, fav }));
  });
};
