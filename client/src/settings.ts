import { ADD_EVENT_LISTENER, ONCLICK, w } from "./alias";
import { updateCurrentTheme, updateTheme, updateThemeCss } from "./helpers";

declare const conf_btn: HTMLButtonElement;
declare const conf_dial: HTMLDialogElement;
declare const conf_reset: HTMLButtonElement;
declare const conf_save: HTMLButtonElement;
declare const dialog_close: HTMLButtonElement;
declare const theme_select: HTMLSelectElement;

let newTheme = "";

conf_btn[ONCLICK] = async () => {
  conf_dial.open = true;

  const currentTheme = await invoke("get_theme_name");
  const themes = (await invoke("list_themes")).split(",").toSorted();

  newTheme = currentTheme;

  const themeName = (t: string) => t.replace(/\s/, "");

  theme_select.innerHTML = themes
    .map((t) => {
      const theme_val = themeName(t);
      const theme_name = t.replace(".toml", "");

      return /* HTML */ `<option value="${theme_val}">${theme_name}</option>`;
    })
    .join("");

  if (currentTheme) theme_select.value = themeName(currentTheme);
};

theme_select.onchange = async () => {
  newTheme = theme_select.value;
  const css = await invoke("preview_theme", theme_select.value);

  updateThemeCss(css);
};

conf_dial[ONCLICK] = (e) => {
  const tags = ["INPUT", "SELECT", "BUTTON"];
  if (tags.includes((e.target as HTMLElement)?.tagName)) return;

  const cname = (e.target as HTMLElement)?.className ?? "";
  if (cname === "dialog-shell" || !cname) {
    updateCurrentTheme();
    conf_dial.open = false;
  }
};

dialog_close[ONCLICK] = () => {
  updateCurrentTheme();
  conf_dial.open = false;
};

conf_reset[ONCLICK] = () => (conf_dial.open = false);
conf_save[ONCLICK] = () => {
  updateTheme(newTheme);
  conf_dial.open = false;
};

w[ADD_EVENT_LISTENER]("keydown", (e) => {
  if (conf_dial.open && e.key === "Escape") {
    updateCurrentTheme();
    conf_dial.open = false;
  }
});
