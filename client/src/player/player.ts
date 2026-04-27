import { ADD_EVENT_LISTENER, KEYDOWN, w } from "../alias";
import { isFocusElement } from "../helpers";
import { PreviewHandler } from "../preview/preview";

export const PAUSED = 0 as const;
export const PLAYING = 1 as const;
export const STOPPED = 2 as const;

export type PlayerState = typeof PAUSED | typeof PLAYING | typeof STOPPED;

declare const time_cur: HTMLSpanElement;
declare const time_est: HTMLSpanElement;

declare const pause_btn: HTMLButtonElement;

function createPlayerHandle() {
  let playerState = 2;
  let intervalId = -1;

  function startTicker(ms = 100) {
    if (intervalId > -1) return;

    intervalId = w.setInterval(async () => {
      const pos = await invoke("get_audio_position");
      const [fmtCur, fmtEst] = (await invoke("get_audio_position_pretty")).split("/");

      time_cur.textContent = fmtCur;
      time_est.textContent = fmtEst;

      PreviewHandler.position = +pos;
    }, ms);
  }

  function stopTicker() {
    clearInterval(intervalId);
    intervalId = -1;
  }

  async function startPlaying(path: string, name: string, tagsList?: string[]) {
    invoke("read_audio_file", path);
    invoke("play_audio_file", path).then(() => {
      playerState = PLAYING;
      startTicker();

      return invoke("get_estimated_len_ms");
    });

    const tags = tagsList ? tagsList : (await invoke("tag_path", path)).split(",");

    PreviewHandler.label = name;
    PreviewHandler.img = "";
    PreviewHandler.tags = tags;
  }

  function pause() {
    invoke("player_pause").then(() => {
      playerState = PAUSED;
      pause_btn.textContent = "Resume";
      stopTicker();
    });
  }

  function resume() {
    invoke("player_resume").then(() => {
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

  w[ADD_EVENT_LISTENER](KEYDOWN, (e) => {
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
      invoke("player_seek", pos as unknown as string);
    },
  };
}

export const playerHandle = createPlayerHandle();
