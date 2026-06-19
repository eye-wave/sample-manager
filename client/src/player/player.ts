import { w } from "../alias";
import { getSampleMetedata } from "../api";
import { IPC, invoke } from "../invoke/invoke";
import { addShortcut } from "../shortcuts";
import { makeSlider } from "./sliders";
import { onEof } from "@state";
import type { Writable } from "../store";

export const PAUSED = 0 as const;
export const PLAYING = 1 as const;
export const STOPPED = 2 as const;

export type PlayerState = typeof PAUSED | typeof PLAYING | typeof STOPPED;

const RESUME_ICON = `<path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z"/>`;
const PAUSE_ICON = `<rect x=14 y=3 width=5 height=18 rx="1"/><rect x=5 y=3 width=5 height=18 rx="1"/>`;

type PlayerStores = {
  playerState: Writable<PlayerState>;
  currentPath: Writable<string | null>;
  position: Writable<number>;
  looping: Writable<boolean>;
  volume: Writable<number>;
  previewMeta: Writable<{ label: string; tags: string[]; fav: boolean }>;
  waveformUri: Writable<string | null>;
};

export const createPlayer = (stores: PlayerStores) => {
  const svg = pause_btn__.querySelector("svg") as SVGElement;

  // --- looping ---
  invoke(IPC.GetLooping).then((v) => stores.looping.set(!!+v));

  stores.looping.subscribe((l) => {
    playback_mode__.textContent = l ? "Loop" : "Oneshot";
  });

  playback_mode__.onclick = () =>
    stores.looping.update((l) => {
      const next = !l;
      invoke(IPC.SetLooping, next ? "1" : "0");
      return next;
    });

  // --- playback icon ---
  stores.playerState.subscribe((s) => {
    svg.innerHTML = s === PLAYING ? PAUSE_ICON : RESUME_ICON;
  });

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
      stores.position.set(pos < lastPos - 0.05 ? 0 : pos);
      lastPos = pos;
    }, ms);
  };

  const stopTicker = () => {
    clearInterval(intervalId);
    intervalId = -1;
  };

  onEof.on(() => {
    stores.playerState.set(STOPPED);
    stores.position.set(1);
    stopTicker();
  });

  // --- volume ---
  makeSlider({
    ctrl: volume_ctrl__,
    txt: volume_txt__,
    max: 2.01,
    norm: (v) => (3 * Math.atan(v)) / Math.PI,
    denorm: (v) => Math.tan((Math.PI * v) / 3),
    get: () => invoke(IPC.GetVolume),
    set: (v) => {
      stores.volume.set(v);
      invoke(IPC.SetVolume, v);
    },
    format: (v) => ((+v * 100) | 0) + "%",
    color: (v) => (v > 1 ? "var(--accent)" : "var(--text-primary)"),
  });

  // --- playback ---
  const startPlaying = async (inpath?: string) => {
    const path = inpath ?? stores.currentPath.get();
    if (!path) return;

    if (inpath) {
      stores.currentPath.set(inpath);
      const meta = await getSampleMetedata(inpath);
      stores.previewMeta.set({ label: meta.name, tags: meta.tags, fav: false });
      stores.waveformUri.set(null);
      stores.position.set(0);
      invoke(IPC.DrawAudioFile, path);
    }

    await invoke(IPC.PlayAudioFile, path);
    stores.playerState.set(PLAYING);
    startTicker();
  };

  const pause = () =>
    invoke(IPC.PlayerPause).then(() => {
      stores.playerState.set(PAUSED);
      stopTicker();
    });

  const resume = () =>
    invoke(IPC.PlayerResume).then(() => {
      stores.playerState.set(PLAYING);
      startTicker();
    });

  const seek = async (pos: number) => {
    const s = stores.playerState.get();
    if (s === PAUSED) await resume();
    if (s === STOPPED) await startPlaying();
    invoke(IPC.PlayerSeek, pos);
  };

  const onClick = () => {
    const s = stores.playerState.get();
    if (s === PAUSED) resume();
    else if (s === PLAYING) pause();
    else if (stores.currentPath.get()) startPlaying();
  };

  pause_btn__.onclick = onClick;
  addShortcut("Toggle play/pause", " ", 0, (e) => {
    if (e.target === volume_ctrl__) return;
    e.preventDefault();
    onClick();
  });

  return { play: startPlaying, pause, seek };
};
