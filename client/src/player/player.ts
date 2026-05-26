import { w } from "../alias";
import * as IPC from "../gen/ipc-gen";
import { invoke, listen } from "../invoke/invoke";
import { PreviewHandler } from "../preview/preview";
import { addShortcut } from "../shortcuts";
import { makeSlider } from "./sliders";

export const PAUSED = 0 as const;
export const PLAYING = 1 as const;
export const STOPPED = 2 as const;

export type PlayerState = typeof PAUSED | typeof PLAYING | typeof STOPPED;

declare const time_cur__: HTMLSpanElement;
declare const time_est__: HTMLSpanElement;

declare const pause_btn__: HTMLButtonElement;

declare const volume_ctrl__: HTMLInputElement;
declare const volume_txt__: HTMLSpanElement;

declare const playback_mode__: HTMLSpanElement;

const RESUME_ICON = `<path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z"/>`;
const PAUSE_ICON = `<rect x=14 y=3 width=5 height=18 rx="1"/><rect x=5 y=3 width=5 height=18 rx="1"/>`;

const getPlaybackMode = async () => !!+(await invoke(IPC.GET_LOOPING));

async function updatePlaybackMode(looping: boolean): Promise<boolean> {
  playback_mode__.textContent = looping ? "Loop" : "Oneshot";
  return looping;
}

getPlaybackMode().then(updatePlaybackMode);
playback_mode__.onclick = () => {
  getPlaybackMode().then((m) => {
    updatePlaybackMode(!m);
    invoke(IPC.SET_LOOPING, !m ? "1" : "0");
  });
};

function createPlayerHandle() {
  const playerState = (() => {
    let playerState: PlayerState = STOPPED;

    return {
      get() {
        return playerState;
      },
      set(v: PlayerState) {
        playerState = v;
        svg.innerHTML = v === PLAYING ? PAUSE_ICON : RESUME_ICON;
      },
    };
  })();

  let intervalId = -1;

  listen("a-eof", () => playerState.set(STOPPED));

  makeSlider({
    ctrl: volume_ctrl__,
    txt: volume_txt__,
    max: 2.01,

    norm: (value: number) => (3 * Math.atan(value)) / Math.PI,
    denorm: (value: number) => Math.tan((Math.PI * value) / 3),

    get: () => invoke(IPC.GET_VOLUME),
    set: (v) => invoke(IPC.SET_VOLUME, v),

    format: (v) => ((+v * 100) | 0) + "%",
    color: (v) => (v > 1 ? "var(--accent)" : "var(--text-primary)"),
  });

  const svg = pause_btn__.querySelector("svg") as SVGElement;
  playerState.set(STOPPED);

  function startTicker(ms = 100) {
    if (intervalId > -1) return;

    let lastPos = 0;

    intervalId = w.setInterval(async () => {
      const pos = +(await invoke(IPC.GET_AUDIO_POSITION));
      const [fmtCur, fmtEst] = (await invoke(IPC.GET_AUDIO_POSITION_PRETTY)).split("/");

      time_cur__.textContent = fmtCur;
      time_est__.textContent = fmtEst;

      if (pos < lastPos - 0.05) {
        PreviewHandler.position = 0;
      } else {
        PreviewHandler.position = pos;
      }

      lastPos = pos;
    }, ms);
  }

  function stopTicker() {
    clearInterval(intervalId);
    intervalId = -1;
  }

  async function startPlaying(
    inpath?: string,
    name?: string,
    fav?: boolean,
    tagsList?: string[],
  ) {
    if (inpath) PreviewHandler.path = inpath;
    if (!PreviewHandler.path) return;

    const path = inpath ?? PreviewHandler.path;

    if (inpath) {
      if (name) PreviewHandler.label = name;
      PreviewHandler.img = "";
      PreviewHandler.position = 0;

      invoke(IPC.DRAW_AUDIO_FILE, path);

      const tags = tagsList ? tagsList : (await invoke(IPC.TAG_PATH, path)).split(",");
      const isFav =
        fav === undefined ? (await invoke(IPC.IS_SAMPLE_FAV, path)) === "true" : fav;

      PreviewHandler.tags = tags;
      PreviewHandler.fav = isFav;
    }

    await invoke(IPC.PLAY_AUDIO_FILE, path).then(() => {
      playerState.set(PLAYING);

      startTicker();
    });
  }

  function pause() {
    invoke(IPC.PLAYER_PAUSE).then(() => {
      playerState.set(PAUSED);
      svg.innerHTML = RESUME_ICON;
      stopTicker();
    });
  }

  async function resume() {
    await invoke(IPC.PLAYER_RESUME).then(() => {
      playerState.set(PLAYING);
      svg.innerHTML = PAUSE_ICON;
      startTicker();
    });
  }

  function onClick() {
    const state = playerState.get();

    if (state === PAUSED) resume();
    else if (state === PLAYING) pause();
    else {
      const p = PreviewHandler.path;
      p && startPlaying();
    }
  }

  async function seek(pos: number) {
    const state = playerState.get();

    if (state === PAUSED) await resume();
    else if (state === STOPPED) await startPlaying();

    invoke(IPC.PLAYER_SEEK, pos);
  }

  pause_btn__.onclick = onClick;

  addShortcut("Toggle play/pause", " ", 0, (e) => {
    if (e.target === volume_ctrl__) return;

    e.preventDefault();
    onClick();
  });

  return {
    get state() {
      return playerState;
    },
    startPlaying,
    pause,
    seek,
  };
}

export const playerHandle = createPlayerHandle();
