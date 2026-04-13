export function basename(name: string) {
  return name.split(/[\\/]/).at(-1) ?? name;
}
