export function decodeAudio(data: string): Uint8Array {
  const buffer = new Uint8Array(data.length);

  for (let i = 0; i < data.length; i++) {
    const sample = resample(data.charCodeAt(i));

    buffer[i] = sample;
  }

  return buffer;
}

export function resample(value: number) {
  let v = value;

  if (v > 92) v -= 1;

  const index = v - 32;
  const t = index / 92;

  return Math.round(t * 255);
}
