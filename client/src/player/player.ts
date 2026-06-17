import { w } from "../alias";
import { getSampleMetedata } from "../api";
import { IPC, invoke, listen } from "../invoke/invoke";
import { PreviewHandler } from "../preview/preview";
import { addShortcut } from "../shortcuts";
import { makeSlider } from "./sliders";

export const PAUSED = 0 as const;
export const PLAYING = 1 as const;
export const STOPPED = 2 as const;

export type PlayerState = typeof PAUSED | typeof PLAYING | typeof STOPPED;

const RESUME_ICON = `<path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z"/>`;
const PAUSE_ICON = `<rect x=14 y=3 width=5 height=18 rx="1"/><rect x=5 y=3 width=5 height=18 rx="1"/>`;

function createPlayerHandle() {
  // --- callbacks registered via onPlay() ---
  const playCallbacks = new Set<(path: string) => void>();

  // --- looping ---
  const getLooping = () => invoke(IPC.GetLooping).then((v) => !!+v);

  getLooping().then((looping) => {
    playback_mode__.textContent = looping ? "Loop" : "Oneshot";
  });

  playback_mode__.onclick = () => {
    getLooping().then((m) => {
      const next = !m;
      playback_mode__.textContent = next ? "Loop" : "Oneshot";
      invoke(IPC.SetLooping, next ? "1" : "0");
    });
  };

  // --- state ---
  const svg = pause_btn__.querySelector("svg") as SVGElement;

  let _state: PlayerState = STOPPED;
  const setState = (v: PlayerState) => {
    _state = v;
    svg.innerHTML = v === PLAYING ? PAUSE_ICON : RESUME_ICON;
  };
  setState(STOPPED);

  // --- ticker ---
  let intervalId = -1;

  const startTicker = (ms = 100) => {
    if (intervalId > -1) return;
    let lastPos = 0;
    intervalId = w.setInterval(async () => {
      const pos = +(await invoke(IPC.GetAudioPosition));
      const [fmtCur, fmtEst] = (await invoke(IPC.GetAudioPositionPretty)).split("/");
      time_cur__.textContent = fmtCur;
      time_est__.textContent = fmtEst;
      PreviewHandler.position = pos < lastPos - 0.05 ? 0 : pos;
      lastPos = pos;
    }, ms);
  };

  const stopTicker = () => {
    clearInterval(intervalId);
    intervalId = -1;
  };

  listen("a-eof", () => {
    setState(STOPPED);
    stopTicker();
    PreviewHandler.position = 1;
  });

  // --- volume ---
  makeSlider({
    ctrl: volume_ctrl__,
    txt: volume_txt__,
    max: 2.01,
    norm: (v) => (3 * Math.atan(v)) / Math.PI,
    denorm: (v) => Math.tan((Math.PI * v) / 3),
    get: () => invoke(IPC.GetVolume),
    set: (v) => invoke(IPC.SetVolume, v),
    format: (v) => ((+v * 100) | 0) + "%",
    color: (v) => (v > 1 ? "var(--accent)" : "var(--text-primary)"),
  });

  // --- core playback ---
  async function startPlaying(inpath?: string) {
    if (inpath) PreviewHandler.path = inpath;
    if (!PreviewHandler.path) return;

    const path = inpath ?? PreviewHandler.path;

    if (inpath) {
      const meta = await getSampleMetedata(inpath);
      PreviewHandler.label = meta.name;
      PreviewHandler.img = "";
      PreviewHandler.position = 0;
      PreviewHandler.tags = meta.tags;
      invoke(IPC.DrawAudioFile, path);
    }

    await invoke(IPC.PlayAudioFile, path);
    setState(PLAYING);
    startTicker();

    for (const cb of playCallbacks) cb(path);
  }

  const pause = () =>
    invoke(IPC.PlayerPause).then(() => {
      setState(PAUSED);
      stopTicker();
    });

  const resume = () =>
    invoke(IPC.PlayerResume).then(() => {
      setState(PLAYING);
      startTicker();
    });

  const seek = async (pos: number) => {
    if (_state === PAUSED) await resume();
    if (_state === STOPPED) await startPlaying();
    invoke(IPC.PlayerSeek, pos);
  };

  const onClick = () => {
    if (_state === PAUSED) resume();
    else if (_state === PLAYING) pause();
    else PreviewHandler.path && startPlaying();
  };

  pause_btn__.onclick = onClick;
  addShortcut("Toggle play/pause", " ", 0, (e) => {
    if (e.target === volume_ctrl__) return;
    e.preventDefault();
    onClick();
  });

  return {
    get state(): PlayerState {
      return _state;
    },
    /** Register a callback fired whenever a new file starts playing. */
    onPlay(cb: (path: string) => void): () => void {
      playCallbacks.add(cb);
      return () => playCallbacks.delete(cb);
    },
    play: startPlaying,
    pause,
    seek,
  };
}

export const playerHandle = createPlayerHandle();
