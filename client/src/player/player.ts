import { w } from "../alias";
import { getSampleMetedata } from "../api";
import { BusEvent, on } from "../bus";
import { invoke, IPC, listen } from "../invoke/invoke";
import { PreviewHandler } from "../preview/preview";
import { addShortcut } from "../shortcuts";
import { makeSlider } from "./sliders";

export const PAUSED = 0 as const;
export const PLAYING = 1 as const;
export const STOPPED = 2 as const;

export type PlayerState = typeof PAUSED | typeof PLAYING | typeof STOPPED;

const RESUME_ICON = `<path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z"/>`;
const PAUSE_ICON = `<rect x=14 y=3 width=5 height=18 rx="1"/><rect x=5 y=3 width=5 height=18 rx="1"/>`;

const getPlaybackMode = async () => !!+(await invoke(IPC.GetLooping));

async function updatePlaybackMode(looping: boolean): Promise<boolean> {
  playback_mode__.textContent = looping ? "Loop" : "Oneshot";
  return looping;
}

getPlaybackMode().then(updatePlaybackMode);
playback_mode__.onclick = () => {
  getPlaybackMode().then((m) => {
    updatePlaybackMode(!m);
    invoke(IPC.SetLooping, !m ? "1" : "0");
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

  listen("a-eof", () => {
    playerState.set(STOPPED);
    stopTicker();
    PreviewHandler.position = 1;
  });

  makeSlider({
    ctrl: volume_ctrl__,
    txt: volume_txt__,
    max: 2.01,

    norm: (value: number) => (3 * Math.atan(value)) / Math.PI,
    denorm: (value: number) => Math.tan((Math.PI * value) / 3),

    get: () => invoke(IPC.GetVolume),
    set: (v) => invoke(IPC.SetVolume, v),

    format: (v) => ((+v * 100) | 0) + "%",
    color: (v) => (v > 1 ? "var(--accent)" : "var(--text-primary)"),
  });

  const svg = pause_btn__.querySelector("svg") as SVGElement;
  playerState.set(STOPPED);

  function startTicker(ms = 100) {
    if (intervalId > -1) return;

    let lastPos = 0;

    intervalId = w.setInterval(async () => {
      const pos = +(await invoke(IPC.GetAudioPosition));
      const [fmtCur, fmtEst] = (await invoke(IPC.GetAudioPositionPretty)).split("/");

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

  async function startPlaying(inpath?: string) {
    if (inpath) PreviewHandler.path = inpath;
    if (!PreviewHandler.path) return;

    const path = inpath ?? PreviewHandler.path;

    if (inpath) {
      const meta = await getSampleMetedata(inpath);
      PreviewHandler.label = meta.name;
      PreviewHandler.img = "";
      PreviewHandler.position = 0;

      invoke(IPC.DrawAudioFile, path);

      PreviewHandler.tags = meta.tags;
    }

    await invoke(IPC.PlayAudioFile, path).then(() => {
      playerState.set(PLAYING);

      startTicker();
    });
  }

  function pause() {
    invoke(IPC.PlayerPause).then(() => {
      playerState.set(PAUSED);
      svg.innerHTML = RESUME_ICON;
      stopTicker();
    });
  }

  async function resume() {
    await invoke(IPC.PlayerResume).then(() => {
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

    invoke(IPC.PlayerSeek, pos);
  }

  pause_btn__.onclick = onClick;

  addShortcut("Toggle play/pause", " ", 0, (e) => {
    if (e.target === volume_ctrl__) return;

    e.preventDefault();
    onClick();
  });

  on(BusEvent.PlaySong, (path: string) => startPlaying(path));

  return {
    get state() {
      return playerState;
    },
    pause,
    seek,
  };
}

export const playerHandle = createPlayerHandle();
