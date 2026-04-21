import { ADD_EVENT_LISTENER, ONCLICK, w } from "./alias";

declare const conf_btn: HTMLButtonElement;
declare const conf_dial: HTMLDialogElement;
declare const conf_reset: HTMLButtonElement;
declare const conf_save: HTMLButtonElement;
declare const dialog_close: HTMLButtonElement;

conf_btn[ONCLICK] = () => (conf_dial.open = true);
conf_dial[ONCLICK] = (e) => {
  if ((e.target as HTMLElement)?.tagName === "INPUT") return;

  const cname = (e.target as HTMLElement)?.className ?? "";
  if (cname === "dialog-shell" || !cname) {
    conf_dial.open = false;
  }
};

dialog_close[ONCLICK] = () => (conf_dial.open = false);

conf_reset[ONCLICK] = () => (conf_dial.open = false);
conf_save[ONCLICK] = () => (conf_dial.open = false);

w[ADD_EVENT_LISTENER]("keydown", (e) => {
  if (conf_dial.open && e.key === "Escape") conf_dial.open = false;
});
