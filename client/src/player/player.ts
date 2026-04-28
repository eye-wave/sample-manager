import { w } from "../alias";
import * as IPC from "../gen/ipc-gen";
import { isFocusElement } from "../helpers";
import { invoke } from "../invoke/invoke";
import { PreviewHandler } from "../preview/preview";

export const PAUSED = 0 as const;
export const PLAYING = 1 as const;
export const STOPPED = 2 as const;

export type PlayerState = typeof PAUSED | typeof PLAYING | typeof STOPPED;

declare const time_cur__: HTMLSpanElement;
declare const time_est__: HTMLSpanElement;

declare const pause_btn: HTMLButtonElement;

function createPlayerHandle() {
  let playerState = 2;
  let intervalId = -1;

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

  async function startPlaying(path: string, name: string, tagsList?: string[]) {
    invoke(IPC.READ_AUDIO_FILE, path);
    invoke(IPC.PLAY_AUDIO_FILE, path).then(() => {
      playerState = PLAYING;
      startTicker();
    });

    const tags = tagsList ? tagsList : (await invoke(IPC.TAG_PATH, path)).split(",");

    PreviewHandler.label = name;
    PreviewHandler.img = "";
    PreviewHandler.tags = tags;
  }

  function pause() {
    invoke(IPC.PLAYER_PAUSE).then(() => {
      playerState = PAUSED;
      pause_btn.textContent = "Resume";
      stopTicker();
    });
  }

  function resume() {
    invoke(IPC.PLAYER_RESUME).then(() => {
      playerState = PLAYING;
      pause_btn.textContent = "Pause";
      startTicker();
    });
  }

  function togglePause() {
    if (playerState === PAUSED) resume();
    else if (playerState === PLAYING) pause();
  }

  pause_btn.onclick = togglePause;

  w.addEventListener("keydown", (e) => {
    if (isFocusElement(e.target)) return;
    if (e.key === " ") togglePause();
  });

  return {
    get state() {
      return playerState;
    },
    startPlaying,
    pause,
    seek(pos: number) {
      if (playerState === PAUSED) resume();
      invoke(IPC.PLAYER_SEEK, pos as unknown as string);
    },
  };
}

export const playerHandle = createPlayerHandle();
