// biome-ignore-start lint/style/noNonNullAssertion: just trust me on this one

import { decodeAudio } from "./decode";

declare const waveCanvas: HTMLCanvasElement;
declare const playBtn: HTMLButtonElement;

let playing = false;
playBtn.addEventListener("click", function () {
  playing = !playing;
  this.textContent = playing ? "⏸" : "▶";
});

const canvas = waveCanvas;
const wrap = canvas.parentElement!;

listen("read_audio", (data) => {
  const audio = decodeAudio(data);
  drawWaveform(audio);
});

const cssRoot = getComputedStyle(document.documentElement);

const gl = canvas.getContext("webgl2")!;

if (!gl) {
  const div = document.createElement("div");
  div.textContent = "webgl not supported";

  canvas.replaceWith(div);
}

const vsSource = `attribute float a_col;attribute vec2 a_extent;varying float v_t;void main(){gl_Position=vec4(a_col*2.-1.,a_extent.x,0,1);v_t=a_extent.y;}`;
const fsSource = `precision mediump float;uniform vec3 u_colorA,u_colorB;varying float v_t;void main(){gl_FragColor=vec4(mix(u_colorB,u_colorA,v_t),1);}`;

function compileShader(type: number, src: string): WebGLShader {
  const s = gl.createShader(type)!;

  gl.shaderSource(s, src);
  gl.compileShader(s);

  return s;
}

const prog = gl.createProgram()!;
gl.attachShader(prog, compileShader(gl.VERTEX_SHADER, vsSource));
gl.attachShader(prog, compileShader(gl.FRAGMENT_SHADER, fsSource));
gl.linkProgram(prog);
gl.useProgram(prog);

const aCol = gl.getAttribLocation(prog, "a_col");
const aExtent = gl.getAttribLocation(prog, "a_extent");
const uColorA = gl.getUniformLocation(prog, "u_colorA")!;
const uColorB = gl.getUniformLocation(prog, "u_colorB")!;

const buf = gl.createBuffer()!;
gl.bindBuffer(gl.ARRAY_BUFFER, buf);

const STRIDE = 3 * 4;
gl.enableVertexAttribArray(aCol);
gl.vertexAttribPointer(aCol, 1, gl.FLOAT, false, STRIDE, 0);
gl.enableVertexAttribArray(aExtent);
gl.vertexAttribPointer(aExtent, 2, gl.FLOAT, false, STRIDE, 4);

function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 0xff) / 255, ((n >> 8) & 0xff) / 255, (n & 0xff) / 255];
}

function drawWaveform(data: Uint8Array) {
  if (!gl) return;

  const waveAColor = cssRoot.getPropertyValue("--wave-a").trim();
  const waveBColor = cssRoot.getPropertyValue("--wave-b").trim();

  const W = wrap.offsetWidth || 900;
  const H = 72;
  canvas.width = W * devicePixelRatio;
  canvas.height = H * devicePixelRatio;
  canvas.style.width = "100%";
  canvas.style.height = `${H}px`;

  gl.viewport(0, 0, canvas.width, canvas.height);
  gl.clearColor(0, 0, 0, 0);
  gl.clear(gl.COLOR_BUFFER_BIT);

  gl.uniform3fv(uColorA, hexToRgb(waveAColor));
  gl.uniform3fv(uColorB, hexToRgb(waveBColor));

  const verts = new Float32Array(W * 6);

  for (let col = 0; col < W; col++) {
    const startIdx = Math.floor((col / W) * data.length);
    const endIdx = Math.max(startIdx + 1, Math.floor(((col + 1) / W) * data.length));

    let min = 128,
      max = 128;
    for (let j = startIdx; j < endIdx; j++) {
      const s = data[j]!;
      if (s < min) min = s;
      if (s > max) max = s;
    }

    const mid = H / 2;
    const yTop = mid + ((max - 128) / 128) * mid * 0.9;
    const yBot = mid + ((min - 128) / 128) * mid * 0.9;

    const xNorm = (col + 0.5) / W;
    const yTopN = 1.0 - (yTop / H) * 2.0;
    const yBotN = 1.0 - (yBot / H) * 2.0;

    const i = col * 6;
    verts[i + 0] = xNorm;
    verts[i + 1] = yBotN;
    verts[i + 2] = 0.0;
    verts[i + 3] = xNorm;
    verts[i + 4] = yTopN;
    verts[i + 5] = 1.0;
  }

  gl.bindBuffer(gl.ARRAY_BUFFER, buf);
  gl.bufferData(gl.ARRAY_BUFFER, verts, gl.DYNAMIC_DRAW);
  gl.vertexAttribPointer(aCol, 1, gl.FLOAT, false, STRIDE, 0);
  gl.vertexAttribPointer(aExtent, 2, gl.FLOAT, false, STRIDE, 4);

  gl.drawArrays(gl.LINES, 0, W * 2);
}

// biome-ignore-end lint/style/noNonNullAssertion: just trust me on this one
