import { arrayEq, event, shallowEq, writable } from "../store";
import type { SampleEntry } from "@typegen/SampleEntry";
import { STOPPED, type PlayerState } from "../player/player";

// --- player state ---
export const playerState = writable<PlayerState>(STOPPED);
export const currentPath = writable<string | null>(null);
export const position = writable<number>(0);
export const looping = writable<boolean>(false);
export const volume = writable<number>(1);

// --- preview ---
export const waveformUri = writable<string | null>(null);
export const previewMeta = writable<{
  label: string;
  tags: string[];
  fav: boolean;
}>({ label: "", tags: [], fav: false }, shallowEq);

// --- browse ---
export const searchResults = writable<SampleEntry[]>([], arrayEq);
export const resultCount = writable<number>(0);

// --- ui ---
export const sidebarWidth = writable<number>(280);
export const activeDialog = writable<string | null>(null);

// --- events ---
export const onFavChanged = event<{ path: string; fav: boolean }>();
export const onSearchResults = event<{ files: SampleEntry[]; count: number }>();
export const onEof = event<void>();
export const onWaveformReady = event<string>();
export const onScanTick = event<string>();
export const onPluginAdded = event<void>();
export const onPluginRemoved = event<void>();
export const onPluginDownload = event<string>();
export const onPluginInfo = event<string>();
export const onLog = event<string>();
