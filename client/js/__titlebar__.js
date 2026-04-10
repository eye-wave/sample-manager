titlebar_handle.addEventListener("mousedown", () => {
  invoke("drag_window");
});

function minimize() {
  invoke("minimize_window");
}
function maximize() {
  invoke("maximize_window");
}
function closeWindow() {
  invoke("close_window");
}
