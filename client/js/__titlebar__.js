titlebar_handle.addEventListener("mousedown", () => {
  invoke("drag");
});

function minimize() {
  invoke("minimize");
}
function maximize() {
  invoke("maximize");
}
function closeWindow() {
  invoke("closeWindow");
}
