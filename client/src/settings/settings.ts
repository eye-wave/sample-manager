import type { AppConfig } from "@typegen/AppConfig";
import type { PickFileOptions } from "@typegen/PickFileOptions";
import type { PluginInfo } from "@typegen/PluginInfo";
import { d, JsonParse, w } from "../alias";
import { getConfigFields } from "../api";
import { capitalize, updateCurrentTheme, updateTheme, updateThemeCss } from "../helpers";
import { IPC, invoke } from "../invoke/invoke";
import { iterateShortcuts } from "../shortcuts";
import { resizeHandle } from "../sidebar/resize";
import { onPluginAdded, onPluginInfo } from "@state";
import { createPluginCard, renderSettings } from "./template";
import type { DialogManager } from "../dialog";
import type { Writable } from "../store";

function createPatch() {
  type Patch = Partial<AppConfig>;
  let patch: Patch = {};
  return {
    flush() {
      patch = {};
    },
    set<F extends keyof AppConfig>(field: F, value: AppConfig[F]) {
      patch[field] = value;
    },
    send() {
      return JSON.stringify(patch);
    },
  };
}

export const createSettings = (sidebarWidth: Writable<number>, dialogs: DialogManager) => {
  const patch = createPatch();
  const tabIds: string[] = [];
  const tabBtns: HTMLButtonElement[] = [];

  d.querySelectorAll(".dialog-body button").forEach((el) => {
    const btn = el as HTMLButtonElement;
    const target = btn.dataset.target;
    if (target) {
      btn.onclick = () => showPane(target);
      tabIds.push(target);
      tabBtns.push(btn);
    }
  });

  tabIds.push("dial_tab_plugin__");

  function showPane(target: string) {
    dial_current_plug_btn__.style.display = "none";
    for (const id of tabIds) {
      // @ts-expect-error
      const el = w[id] as HTMLDivElement;
      el.style.display = id === target ? "contents" : "none";
    }
    for (const btn of tabBtns) {
      btn.blur();
      btn.classList.toggle("active", btn.dataset.target === target);
    }
  }

  const refreshPluginList = () => invoke(IPC.GetAllPluginsInfo);
  let previewedTheme = "";

  conf_btn__.onclick = async () => {
    conf_btn__.blur();
    dialogs.open("conf_dial__");
    patch.flush();
    refreshPluginList();

    try {
      const config = await getConfigFields();
      renderSettings({ id: "__APP_SETTINGS__", config }, settings_body__, (field, data) => {
        if (field === "ffmpeg_path") patch.set("ffmpeg_path", data as string);
        else if (field === "ffprobe_path") patch.set("ffprobe_path", data as string);
        else if (field === "sidebar_width") {
          const width = +(data as string);
          patch.set("sidebar_width", width);
          sidebarWidth.set(width);
          resizeHandle(width);
        } else if (field === "tracked_dirs") patch.set("tracked_dirs", data as string[]);
        else if (field === "color_theme") {
          previewedTheme = data as string;
          invoke(IPC.PreviewTheme, data).then(updateThemeCss);
        }
      });
    } catch (_) {}

    dial_tab_shortcuts_body__.innerHTML = "";
    for (const [key, val] of iterateShortcuts()) {
      const bitmask = +key.charAt(0);
      const keys = [key.slice(1)];
      (bitmask & (1 << 2)) !== 0 && keys.push("Ctrl");
      (bitmask & (1 << 1)) !== 0 && keys.push("Shift");
      (bitmask & (1 << 0)) !== 0 && keys.push("Alt");
      const kbd = keys.map((k) => `<kbd>${capitalize(k)}</kbd>`).join(" + ");
      dial_tab_shortcuts_body__.innerHTML += `<div style="color:var(--text-primary)"><span>${val}</span>${" " + kbd}</div>`;
    }

    dial_tab_cache_body__.innerHTML = await invoke(IPC.GetAppCacheSize);
  };

  const close = () => dialogs.close();
  const revertAndClose = () => {
    updateCurrentTheme();
    close();
  };

  conf_dial__.onclick = (e: MouseEvent) => {
    if (e.target === conf_save__) return;
    const {
      offsetLeft: left,
      offsetTop: top,
      offsetWidth: ew,
      offsetHeight: eh,
    } = conf_dial_body__;
    if (e.clientX < left || e.clientX > left + ew || e.clientY < top || e.clientY > top + eh)
      revertAndClose();
  };

  dialog_close__.onclick = revertAndClose;
  conf_reset__.onclick = close;
  conf_save__.onclick = () => {
    if (previewedTheme) updateTheme(previewedTheme);
    invoke(IPC.PatchConfig, patch.send());
    close();
  };

  add_plugin_btn__.onclick = async () => {
    const opt: PickFileOptions = { filters: ["*.zip"], label: "*.zip zipped plugin files" };
    const path = await invoke(IPC.PickFile, opt);
    if (!path) return;
    invoke(IPC.AddPlugin, path);
  };

  const bindPluginCards = (pluginsInfo: PluginInfo[]) => {
    plugins_settings__.innerHTML = pluginsInfo.map((i) => createPluginCard(i)).join("");
    (plugins_settings__.querySelectorAll(".btn") as NodeListOf<HTMLButtonElement>).forEach(
      (btn) => {
        const plugId = btn.dataset.id as string;
        const info = pluginsInfo.find((p) => p.id === plugId);
        if (!info) return;
        btn.onclick = () => {
          showPane("dial_tab_plugin__");
          plugin_uninstall_btn__.dataset.plugin = info.id;
          dial_current_plug_btn__.style.display = "";
          dial_current_plug_btn__.textContent = info.name;
          plugin_settings_label__.textContent = "Plugin " + info.name;
          renderSettings(info, plugin_settings_body__);
        };
      },
    );
  };

  onPluginInfo.on((data) => {
    const pluginsInfo: PluginInfo[] = [];
    try {
      JsonParse(data).forEach((i: PluginInfo) => {
        pluginsInfo.push(i);
      });
    } catch {}
    bindPluginCards(pluginsInfo);
  });

  onPluginAdded.on(refreshPluginList);

  plugin_uninstall_btn__.onclick = () => {
    const id = plugin_uninstall_btn__.dataset.plugin;
    if (!id) return;
    invoke(IPC.UninstallPlugin, id).then(() => {
      showPane("dial_tab_plug__");
      refreshPluginList();
    });
  };
};
