import { w } from "../alias";
import * as IPC from "../gen/ipc-gen";
import { isFocusElement } from "../helpers";
import { invoke } from "../invoke/invoke";
import { PreviewHandler } from "../preview/preview";
import { addShortcut } from "../shortcuts";

export const PAUSED = 0 as const;
export const PLAYING = 1 as const;
export const STOPPED = 2 as const;

export type PlayerState = typeof PAUSED | typeof PLAYING | typeof STOPPED;

declare const time_cur__: HTMLSpanElement;
declare const time_est__: HTMLSpanElement;

declare const pause_btn__: HTMLButtonElement;

declare const volume_ctrl__: HTMLInputElement;
declare const volume_txt__: HTMLSpanElement;

const RESUME_ICON = `<path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z"/>`;
const PAUSE_ICON = `<rect x=14 y=3 width=5 height=18 rx="1"/><rect x=5 y=3 width=5 height=18 rx="1"/>`;

function createPlayerHandle() {
  const norm = (value: number) => (3 * Math.atan(value)) / Math.PI;
  const denorm = (value: number) => Math.tan((Math.PI * value) / 3);

  volume_ctrl__.max = norm(2.01).toPrecision(4);

  const setVolumeText = (value: number) => {
    volume_txt__.textContent = `${value > 1 ? "+" : ""}${(value * 100) | 0}%`;
    volume_txt__.style.color = value > 1 ? "var(--accent)" : "var(--text-primary)";
  };

  const setVolume = (value: number) => {
    invoke(IPC.SET_VOLUME, value);
    setVolumeText(value);
  };

  const syncSlider = (value: number) => {
    volume_ctrl__.value = norm(value) as unknown as string;
  };

  invoke(IPC.GET_VOLUME).then((value) => {
    const volume = +value;

    setVolumeText(volume);
    syncSlider(volume);
  });

  volume_ctrl__.oninput = () => {
    setVolume(denorm(+volume_ctrl__.value));
  };

  volume_ctrl__.ondblclick = () => {
    const defaultVolume = 1;

    syncSlider(defaultVolume);
    setVolume(defaultVolume);
  };

  let playerState = 2;
  let intervalId = -1;

  const svg = pause_btn__.querySelector("svg") as SVGElement;
  svg.innerHTML = RESUME_ICON;

  function startTicker(ms = 100) {
    if (intervalId > -1) return;

    intervalId = w.setInterval(async () => {
      const pos = await invoke(IPC.GET_AUDIO_POSITION);
      const [fmtCur, fmtEst] = (await invoke(IPC.GET_AUDIO_POSITION_PRETTY)).split("/");

      time_cur__.textContent = fmtCur;
      time_est__.textContent = fmtEst;

      PreviewHandler.position = +pos;
    }, ms);
  }

  function stopTicker() {
    clearInterval(intervalId);
    intervalId = -1;
  }

  async function startPlaying(path: string, name: string, fav?: boolean, tagsList?: string[]) {
    PreviewHandler.path = path;
    PreviewHandler.label = name;
    PreviewHandler.img = "";
    PreviewHandler.position = 0;

    // const savedPath = await invoke(IPC.PLUGIN_DOWNLOAD_FILE, {
    //   id: "plugin-id",
    //   url: PreviewHandler.path,
    // });

    await invoke(IPC.PLAY_AUDIO_FILE, path).then(() => {
      playerState = PLAYING;
      svg.innerHTML = PAUSE_ICON;
      startTicker();
    });
    invoke(IPC.DRAW_AUDIO_FILE, path);

    const tags = tagsList ? tagsList : (await invoke(IPC.TAG_PATH, path)).split(",");
    const isFav = fav === undefined ? (await invoke(IPC.IS_SAMPLE_FAV, path)) === "true" : fav;

    PreviewHandler.tags = tags;
    PreviewHandler.fav = isFav;
  }

  function pause() {
    invoke(IPC.PLAYER_PAUSE).then(() => {
      playerState = PAUSED;
      svg.innerHTML = RESUME_ICON;
      stopTicker();
    });
  }

  function resume() {
    invoke(IPC.PLAYER_RESUME).then(() => {
      playerState = PLAYING;
      svg.innerHTML = PAUSE_ICON;
      startTicker();
    });
  }

  function togglePause() {
    if (playerState === PAUSED) resume();
    else if (playerState === PLAYING) pause();
  }

  pause_btn__.onclick = togglePause;

  addShortcut("Toggle play/pause", " ", 0, (e) => {
    if (e.target === volume_ctrl__) return;

    e.preventDefault();
    togglePause();
  });

  return {
    get state() {
      return playerState;
    },
    startPlaying,
    pause,
    seek(pos: number) {
      if (playerState === PAUSED) resume();
      invoke(IPC.PLAYER_SEEK, pos);
    },
  };
}

export const playerHandle = createPlayerHandle();
