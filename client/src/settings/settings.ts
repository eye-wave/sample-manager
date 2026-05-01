import type { PluginInfo } from "@typegen/PluginInfo";
import { w } from "../alias";
import * as IPC from "../gen/ipc-gen";
import { updateCurrentTheme, updateTheme, updateThemeCss } from "../helpers";
import { invoke } from "../invoke/invoke";
import { createPluginCard } from "./template";

declare const conf_btn__: HTMLButtonElement;
declare const conf_dial__: HTMLDialogElement;
declare const conf_reset__: HTMLButtonElement;
declare const conf_save__: HTMLButtonElement;
declare const dialog_close__: HTMLButtonElement;
declare const theme_select__: HTMLSelectElement;
declare const conf_dial_body__: HTMLDivElement;

declare const plugin_settings__: HTMLDivElement;

let newTheme = "";

const themeName = (t: string) => t.replace(/\s/, "");

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

  const pluginsInfo: PluginInfo[] = await invoke(
    IPC.GET_ALL_PLUGINS_INFO,
    "freesound-search",
  ).then((res) => JSON.parse(res));

  plugin_settings__.innerHTML = pluginsInfo.map((i) => createPluginCard(i)).join("");

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
  conf_dial__.close();
};

w.addEventListener("keydown", (e) => {
  if (conf_dial__.open && e.key === "Escape") {
    updateCurrentTheme();
    conf_dial__.close();
  }
});
