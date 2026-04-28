import { w } from "../alias";
import { isFocusElement, updateCurrentTheme, updateTheme, updateThemeCss } from "../helpers";

declare const conf_btn__: HTMLButtonElement;
declare const conf_dial__: HTMLDialogElement;
declare const conf_reset__: HTMLButtonElement;
declare const conf_save__: HTMLButtonElement;
declare const dialog_close__: HTMLButtonElement;
declare const theme_select__: HTMLSelectElement;

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
  conf_dial__.open = true;

  const currentTheme = await invoke("get_theme_name");
  const [lightCount, ...themes] = (await invoke("list_themes")).split(",");

  const lightThemes = themes.slice(0, +lightCount).toSorted();
  const darkThemes = themes.slice(+lightCount).toSorted();

  newTheme = currentTheme;

  theme_select__.innerHTML =
    themeSelectionTemplate("light", lightThemes) + themeSelectionTemplate("dark", darkThemes);

  if (currentTheme) theme_select__.value = themeName(currentTheme);
};

theme_select__.onchange = async () => {
  newTheme = theme_select__.value;
  const css = await invoke("preview_theme", theme_select__.value);

  updateThemeCss(css);
};

conf_dial__.onclick = (e) => {
  if (isFocusElement(e.target)) return;

  const cname = (e.target as HTMLElement)?.className ?? "";
  if (cname === "dialog-shell" || !cname) {
    updateCurrentTheme();
    conf_dial__.open = false;
  }
};

dialog_close__.onclick = () => {
  updateCurrentTheme();
  conf_dial__.open = false;
};

conf_reset__.onclick = () => (conf_dial__.open = false);
conf_save__.onclick = () => {
  updateTheme(newTheme);
  conf_dial__.open = false;
};

w.addEventListener("keydown", (e) => {
  if (conf_dial__.open && e.key === "Escape") {
    updateCurrentTheme();
    conf_dial__.open = false;
  }
});
