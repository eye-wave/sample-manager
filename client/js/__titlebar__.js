titlebar_handle.addEventListener("mousedown", () => {
  console.log("Poggers");
  window.ipc.postMessage("drag");
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
