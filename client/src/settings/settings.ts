import type { AppConfig } from "@typegen/AppConfig";
import type { PluginInfo } from "@typegen/PluginInfo";
import { d, w } from "../alias";
import * as IPC from "../gen/ipc-gen";
import { updateCurrentTheme, updateTheme, updateThemeCss } from "../helpers";
import { invoke } from "../invoke/invoke";
import type { LooseInput } from "../lying";
import { createPluginCard, renderField } from "./template";

declare const conf_btn__: HTMLButtonElement;
declare const conf_dial__: HTMLDialogElement;
declare const conf_dial_body__: HTMLDivElement;
declare const conf_reset__: HTMLButtonElement;
declare const conf_save__: HTMLButtonElement;
declare const dialog_close__: HTMLButtonElement;
declare const ffmpeg_path__: HTMLInputElement;
declare const plugins_settings__: HTMLDivElement;
declare const sidebar_width__: LooseInput;
declare const theme_select__: HTMLSelectElement;
declare const plugin_settings_label__: HTMLParagraphElement;
declare const plugin_settings_body__: HTMLDivElement;

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

let newTheme = "";

const themeName = (t: string) => t.replace(/\s/, "");

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

    if (id === target) el.style.display = "contents";
    else el.style.display = "none";
  }

  for (const btn of tabBtns) {
    if (btn.dataset.target === target) btn.classList.add("active");
    else btn.classList.remove("active");
  }
}

function themeSelectionTemplate(type: "light" | "dark", themes: string[]) {
  const inner = themes
    .map((t) => {
      const theme_val = themeName(t);
      const theme_name = t.replace(".toml", "");

      return /* HTML */ `<option value="${theme_val}">${theme_name}</option>`;
    })
    .join("");

  return /* HTML */ `<optgroup label="${type}">${inner}</optgroup>`;
}

conf_btn__.onclick = async () => {
  conf_dial__.showModal();
  patch.flush();

  const pluginsInfo: PluginInfo[] = await invoke(IPC.GET_ALL_PLUGINS_INFO).then((res) =>
    JSON.parse(res),
  );

  try {
    const settings: AppConfig = JSON.parse(await invoke(IPC.GET_CONFIG_AS_JSON));

    if (settings.ffmpeg_path) ffmpeg_path__.value = settings.ffmpeg_path;
    if (settings.sidebar_width) sidebar_width__.value = settings.sidebar_width;
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
      plugin_settings_body__.innerHTML = Object.entries(info.config)
        .map(([k, f]) => renderField(k, f))
        .join("");
    };
  });

  const currentTheme = await invoke(IPC.GET_THEME_NAME);
  const [lightCount, ...themes] = (await invoke(IPC.LIST_THEMES)).split(",");

  const lightThemes = themes.slice(0, +lightCount).toSorted();
  const darkThemes = themes.slice(+lightCount).toSorted();

  newTheme = currentTheme;

  theme_select__.innerHTML =
    themeSelectionTemplate("light", lightThemes) + themeSelectionTemplate("dark", darkThemes);

  if (currentTheme) theme_select__.value = themeName(currentTheme);
};

theme_select__.onchange = async () => {
  newTheme = theme_select__.value;
  const css = await invoke(IPC.PREVIEW_THEME, theme_select__.value);

  updateThemeCss(css);
};

conf_dial__.onclick = (e: MouseEvent) => {
  const left = conf_dial_body__.offsetLeft;
  const right = left + conf_dial_body__.offsetWidth;
  const top = conf_dial_body__.offsetTop;
  const bottom = top + conf_dial_body__.offsetHeight;

  const isClickOutside =
    e.clientX < left || e.clientX > right || e.clientY < top || e.clientY > bottom;

  if (isClickOutside) {
    updateCurrentTheme();

    conf_dial__.close();
  }
};

dialog_close__.onclick = () => {
  updateCurrentTheme();
  conf_dial__.close();
};

conf_reset__.onclick = () => conf_dial__.close();
conf_save__.onclick = () => {
  updateTheme(newTheme);
  invoke(IPC.PATCH_CONFIG, patch.send());

  conf_dial__.close();
};

w.addEventListener("keydown", (e) => {
  if (conf_dial__.open && e.key === "Escape") {
    updateCurrentTheme();
    conf_dial__.close();
  }
});

ffmpeg_path__.oninput = () => patch.set("ffmpeg_path", ffmpeg_path__.value);
sidebar_width__.oninput = () => patch.set("sidebar_width", +sidebar_width__.value);
