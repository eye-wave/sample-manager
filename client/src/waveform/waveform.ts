declare const playBtn: HTMLButtonElement;
declare const waveform: HTMLImageElement;

let playing = false;
playBtn.onclick = () => {
  playing = !playing;
  playBtn.textContent = playing ? "⏸" : "▶";
};

listen("read_audio", (uri) => {
  waveform.style.maskImage = `url(${uri})`;
});
