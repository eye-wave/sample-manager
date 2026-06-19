import "./invoke/invoke";
import "./__app__";
import "./state/adapters";

import * as state from "@state";
import { createDialogManager } from "./dialog";
import { createPlayer } from "./player/player";
import { createPreview } from "./preview/preview";
import { createBrowse } from "./browse/browse";
import { createSidebar } from "./sidebar/sidebar";
import { createSettings } from "./settings/settings";
import { createTerminal } from "./terminal/terminal";

const dialogs = createDialogManager(state.activeDialog);

const player = createPlayer({
  playerState: state.playerState,
  currentPath: state.currentPath,
  position: state.position,
  looping: state.looping,
  volume: state.volume,
  previewMeta: state.previewMeta,
  waveformUri: state.waveformUri,
});

createPreview(
  {
    currentPath: state.currentPath,
    position: state.position,
    previewMeta: state.previewMeta,
    waveformUri: state.waveformUri,
  },
  player.seek,
);

const { search } = createBrowse(
  {
    currentPath: state.currentPath,
    searchResults: state.searchResults,
    resultCount: state.resultCount,
  },
  player.play,
);

createSidebar(player.play, search);
createSettings(state.sidebarWidth, dialogs);
createTerminal(dialogs);
