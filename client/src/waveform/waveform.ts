declare const waveCanvas: HTMLCanvasElement;
declare const playBtn: HTMLButtonElement;

(() => {
  const canvas = waveCanvas;
  const wrap = canvas.parentElement!;

  function draw() {
    canvas.width = wrap.offsetWidth || 900;
    canvas.height = 72;
    canvas.style.width = "100%";
    canvas.style.height = "72px";
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const W = canvas.width,
      H = canvas.height,
      mid = H / 2;
    const N = 180;
    const seed = [
      0.12, 0.55, 0.82, 0.38, 0.91, 0.44, 0.67, 0.2, 0.75, 0.88, 0.3, 0.6, 0.95, 0.42, 0.7,
      0.15, 0.5, 0.85, 0.35, 0.65, 0.9, 0.25, 0.78, 0.48, 0.62, 0.19, 0.8, 0.56, 0.33, 0.72,
    ];
    function amp(i: number) {
      const base = seed[i % seed.length] ?? 0;
      const mod = Math.sin(i * 0.18) * 0.18 + Math.sin(i * 0.07) * 0.22;
      return Math.min(0.97, Math.max(0.04, base + mod));
    }
    const playhead = 0.28;
    const barW = (W / N) * 0.55;
    const gap = (W / N) * 0.45;
    ctx.clearRect(0, 0, W, H);
    for (let i = 0; i < N; i++) {
      const x = i * (barW + gap);
      const a = amp(i);
      const h = a * (mid * 1.7);
      const progress = x / W;
      let color;
      if (progress < playhead) {
        const t = progress / playhead;
        const r = Math.round(91 + (196 - 91) * t);
        const g = Math.round(110 + (91 - 110) * t);
        const b = Math.round(245 + (245 - 245) * t);
        color = `rgba(${r},${g},${b},0.9)`;
      } else {
        color = `rgba(80,80,110,0.5)`;
      }
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.roundRect(x, mid - h / 2, barW, h, 2);
      ctx.fill();
    }
  }

  draw();
  window.addEventListener("resize", draw);

  let playing = false;
  playBtn.addEventListener("click", function () {
    playing = !playing;
    this.textContent = playing ? "⏸" : "▶";
  });
})();
