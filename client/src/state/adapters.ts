import { JsonParse } from "../alias";
import { listen } from "../invoke/invoke";
import {
  onEof,
  onFavChanged,
  onPluginAdded,
  onPluginDownload,
  onPluginInfo,
  onPluginRemoved,
  onScanTick,
  onWaveformReady,
  searchResults,
  resultCount,
  onLog,
} from "./state";
import type { SampleEntry } from "@typegen/SampleEntry";

listen("log", (p) => onLog.emit(p));
listen("a-eof", () => onEof.emit());
listen("read_audio", (uri) => onWaveformReady.emit(uri));
listen("s_tick", (n) => onScanTick.emit(n));
listen("set-fav", (payload) => {
  onFavChanged.emit({
    fav: !!+payload.charAt(0),
    path: payload.slice(1),
  });
});

listen("search", (payload) => {
  const { files, count }: { files: SampleEntry[]; count: number } = (() => {
    try {
      return JsonParse(payload);
    } catch {
      return { count: 0, files: [] };
    }
  })();
  searchResults.set(files);
  resultCount.set(count);
});

listen("plugin-info", (data) => onPluginInfo.emit(data));
listen("plug-add", () => onPluginAdded.emit());
listen("plug-rm", () => onPluginRemoved.emit());
listen("plug-download", (path) => onPluginDownload.emit(path));
