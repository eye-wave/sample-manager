import type { AppConfig } from "@typegen/AppConfig";
import type { PickFileOptions } from "@typegen/PickFileOptions";
import type { PluginInfo } from "@typegen/PluginInfo";
import type { SchemaFieldWithValue } from "@typegen/SchemaFieldWithValue";
import { d, w } from "../alias";
import * as IPC from "../gen/ipc-gen";
import { capitalize, updateCurrentTheme, updateTheme, updateThemeCss } from "../helpers";
import { invoke, listen } from "../invoke/invoke";
import { addShortcut, iterateShortcuts } from "../shortcuts";
import { resizeHandle } from "../sidebar/resize";
import { bindSettingInputs } from "./inputs";
import { createPluginCard, renderSettings } from "./template";

declare const add_plugin_btn__: HTMLButtonElement;
declare const conf_btn__: HTMLButtonElement;
declare const conf_dial__: HTMLDialogElement;
declare const conf_dial_body__: HTMLDivElement;
declare const conf_reset__: HTMLButtonElement;
declare const conf_save__: HTMLButtonElement;
declare const dialog_close__: HTMLButtonElement;
declare const plugin_settings_body__: HTMLDivElement;
declare const plugin_settings_label__: HTMLParagraphElement;
declare const plugins_settings__: HTMLDivElement;
declare const settings_body__: HTMLDivElement;
declare const dial_tab_shortcuts__: HTMLDivElement;

function createPatch() {
  type Patch = Partial<AppConfig>;
  type Field = keyof AppConfig;

  let patch: Patch = {};

  return {
    flush() {
      patch = {} as Patch;
    },
    set<F extends Field>(field: F, value: AppConfig[F]) {
      patch[field] = value;
    },
    send() {
      return JSON.stringify(patch);
    },
  };
}

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
  for (const id of tabIds) {
    // @ts-expect-error
    const el = w[id] as HTMLDivElement;

    el.style.display = id === target ? "contents" : "none";
  }

  for (const btn of tabBtns) {
    btn.blur();
    if (btn.dataset.target === target) btn.classList.add("active");
    else btn.classList.remove("active");
  }
}

let previewedTheme = "";

conf_btn__.onclick = async () => {
  conf_btn__.blur();
  conf_dial__.showModal();
  patch.flush();

  invoke(IPC.GET_ALL_PLUGINS_INFO);

  try {
    const config: Record<string, SchemaFieldWithValue> = JSON.parse(
      await invoke(IPC.GET_CONFIG_FIELDS),
    );

    settings_body__.innerHTML = renderSettings({ id: "__APP_SETTINGS__", config });
    bindSettingInputs(settings_body__, (field, data) => {
      if (field === "ffmpeg_path") patch.set("ffmpeg_path", data as string);
      else if (field === "ffprobe_path") patch.set("ffprobe_path", data as string);
      else if (field === "sidebar_width") {
        const width = +(data as string);
        patch.set("sidebar_width", width);
        resizeHandle(width);
      } else if (field === "tracked_dirs") patch.set("tracked_dirs", data as string[]);
      else if (field === "color_theme") {
        previewedTheme = data as string;

        invoke(IPC.PREVIEW_THEME, data).then(updateThemeCss);
      }
    });
  } catch (_) {}

  dial_tab_shortcuts__.innerHTML = "";

  for (const [key, val] of iterateShortcuts()) {
    const bitmask = +key.charAt(0);
    const keys = [key.slice(1)];

    (bitmask & (1 << 2)) !== 0 && keys.push("Ctrl");
    (bitmask & (1 << 1)) !== 0 && keys.push("Shift");
    (bitmask & (1 << 0)) !== 0 && keys.push("Alt");

    const kbd = keys.map((k) => /* HTML */ `<kbd>${capitalize(k)}</kbd>`).join(" + ");

    dial_tab_shortcuts__.innerHTML += /* HTML */ `<div style="color:var(--text-primary)">
      <span>${val}</span>${" " + kbd}
    </div>`;
  }
};

const revertAndClose = () => {
  updateCurrentTheme();
  conf_dial__.close();
};

conf_dial__.onclick = (e: MouseEvent) => {
  if (e.target === conf_save__) return;

  const {
    offsetLeft: left,
    offsetTop: top,
    offsetWidth: w,
    offsetHeight: h,
  } = conf_dial_body__;
  const outside =
    e.clientX < left || e.clientX > left + w || e.clientY < top || e.clientY > top + h;
  if (outside) revertAndClose();
};

dialog_close__.onclick = () => {
  updateCurrentTheme();
  conf_dial__.close();
};

conf_reset__.onclick = () => conf_dial__.close();
conf_save__.onclick = () => {
  if (previewedTheme) updateTheme(previewedTheme);
  invoke(IPC.PATCH_CONFIG, patch.send());
  conf_dial__.close();
};

add_plugin_btn__.onclick = async () => {
  const opt: PickFileOptions = { filters: ["*.zip"], label: "*.zip zipped plugin files" };

  const path = await invoke(IPC.PICK_FILE, opt);
  if (!path) return;
};

addShortcut("Close settings dialog", "Escape", 0, () => {
  if (conf_dial__.open) {
    updateCurrentTheme();
    conf_dial__.close();
  }
});

listen("plugin-info", (data) => {
  const pluginsInfo: PluginInfo[] = [];
  try {
    const items: PluginInfo[] = JSON.parse(data);
    items.forEach((i) => {
      pluginsInfo.push(i);
    });
  } catch {}

  plugins_settings__.innerHTML = pluginsInfo.map((i) => createPluginCard(i)).join("");
  plugins_settings__.querySelectorAll(".btn").forEach((el) => {
    const btn = el as HTMLButtonElement;
    const plugId = btn.dataset.id as string;

    const info = pluginsInfo.find((p) => p.id === plugId);
    if (!info) return;

    btn.onclick = () => {
      showPane("dial_tab_plugin__");

      plugin_settings_label__.textContent = "Plugin " + info.name;
      plugin_settings_body__.innerHTML = renderSettings(info);

      bindSettingInputs(plugin_settings_body__);
    };
  });
});
