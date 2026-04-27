import { ADD_EVENT_LISTENER, KEYDOWN, w } from "../alias";
import { isFocusElement } from "../helpers";
import { PreviewHandler } from "../preview/preview";

export const PAUSED = 0 as const;
export const PLAYING = 1 as const;
export const STOPPED = 2 as const;

export type PlayerState = typeof PAUSED | typeof PLAYING | typeof STOPPED;

function createPlayerHandle() {
  let playerState = 2;
  let intervalId = -1;

  function startTicker(ms = 100) {
    if (intervalId > -1) return;

    intervalId = w.setInterval(async () => {
      const pos = await invoke("get_audio_position");

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
    });

    const tags = tagsList ? tagsList : (await invoke("tag_path", path)).split(",");

    PreviewHandler.label = name;
    PreviewHandler.img = "";
    PreviewHandler.tags = tags;
  }

  function pause() {
    invoke("player_pause").then(() => {
      playerState = PAUSED;
      stopTicker();
    });
  }

  function resume() {
    invoke("player_resume").then(() => {
      playerState = PLAYING;
      startTicker();
    });
  }

  w[ADD_EVENT_LISTENER](KEYDOWN, (e) => {
    if (isFocusElement(e.target)) return;
    if (e.key === " ") {
      if (playerState === PAUSED) resume();
      else if (playerState === PLAYING) pause();
    }
  });

  return {
    get state() {
      return playerState;
    },
    startPlaying,
    pause,
  };
}

export const playerHandle = createPlayerHandle();
