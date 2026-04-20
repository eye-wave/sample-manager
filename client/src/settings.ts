declare const conf_btn: HTMLButtonElement;
declare const conf_dial: HTMLDialogElement;
declare const conf_reset: HTMLButtonElement;
declare const conf_save: HTMLButtonElement;
declare const dialog_close: HTMLButtonElement;

conf_btn.onclick = () => (conf_dial.open = true);
conf_dial.onclick = (e) => {
  if ((e.target as HTMLElement)?.tagName === "INPUT") return;

  const cname = (e.target as HTMLElement)?.className ?? "";
  if (cname === "dialog-shell" || !cname) {
    conf_dial.open = false;
  }
};

dialog_close.onclick = () => (conf_dial.open = false);

conf_reset.onclick = () => (conf_dial.open = false);
conf_save.onclick = () => (conf_dial.open = false);

window.addEventListener("keydown", (e) => {
  if (conf_dial.open && e.key === "Escape") conf_dial.open = false;
});
