import type { AppConfig } from "@typegen/AppConfig";
import type { PluginInfo } from "@typegen/PluginInfo";
import type { SchemaFieldWithValue } from "@typegen/SchemaFieldWithValue";
import { d, w } from "../alias";
import * as IPC from "../gen/ipc-gen";
import { updateCurrentTheme, updateTheme, updateThemeCss } from "../helpers";
import { invoke } from "../invoke/invoke";
import { bindSettingInputs, createPluginCard, renderSettings } from "./template";

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
  const target = btn.dataset.target as string;

  btn.onclick = () => showPane(target);

  tabIds.push(target);
  tabBtns.push(btn);
});

tabIds.push("dial_tab_plugin__");

function showPane(target: string) {
  for (const id of tabIds) {
    // @ts-expect-error
    const el = w[id] as HTMLDivElement;

    el.style.display = id === target ? "contents" : "none";
  }

  for (const btn of tabBtns) {
    if (btn.dataset.target === target) btn.classList.add("active");
    else btn.classList.remove("active");
  }
}

let previewedTheme = "";

conf_btn__.onclick = async () => {
  conf_dial__.showModal();
  patch.flush();

  const pluginsInfo: PluginInfo[] = await invoke(IPC.GET_ALL_PLUGINS_INFO).then((res) =>
    JSON.parse(res),
  );

  try {
    const config: Record<string, SchemaFieldWithValue> = JSON.parse(
      await invoke(IPC.GET_CONFIG_FIELDS),
    );

    settings_body__.innerHTML = renderSettings({ id: "__APP_SETTINGS__", config });
    bindSettingInputs(settings_body__, (field, data) => {
      if (field === "ffmpeg_path") patch.set("ffmpeg_path", data);
      else if (field === "ffprobe_path") patch.set("ffprobe_path", data);
      else if (field === "sidebar_width") patch.set("sidebar_width", +data);
      else if (field === "color_theme") {
        previewedTheme = data;

        invoke(IPC.PREVIEW_THEME, data).then(updateThemeCss);
      }
    });
  } catch (_) {}

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

w.addEventListener("keydown", (e) => {
  if (conf_dial__.open && e.key === "Escape") {
    updateCurrentTheme();
    conf_dial__.close();
  }
});
