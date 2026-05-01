export function missingUri(message: string) {
  return (
    "data:image/svg+xml;utf8," +
    encodeURIComponent(
      `<svg xmlns='http://www.w3.org/2000/svg' width='900' height='256' viewBox='0 0 900 256'><rect width='900' height='256' fill='#000' /><text x='50%' y='50%' fill='#fff' font-family='sans-serif' font-size='50' text-anchor='middle' dominant-baseline='central'>${message}</text></svg>`,
    )
  );
}
