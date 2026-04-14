declare const playBtn: HTMLButtonElement;
declare const waveform: HTMLImageElement;

let playing = false;
playBtn.addEventListener("click", function () {
  playing = !playing;
  this.textContent = playing ? "⏸" : "▶";
});

listen("read_audio", (uri) => {
  waveform.style.maskImage = `url(${uri})`;
});
