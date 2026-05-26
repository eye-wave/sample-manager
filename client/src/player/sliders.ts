export type SliderConfig = {
  ctrl: HTMLInputElement;
  txt: HTMLSpanElement;

  max: number;

  format: (v: string) => string;
  color: (v: number) => string;

  get: () => Promise<string>;
  set: (v: number) => void;

  norm: (v: number) => number;
  denorm: (v: number) => number;
};

export function makeSlider(cfg: SliderConfig) {
  const norm = cfg.norm;
  const denorm = cfg.denorm;

  cfg.ctrl.max = norm(cfg.max).toPrecision(4);

  const setText = (v: string | number) => {
    cfg.txt.textContent = cfg.format(v as unknown as string);
    cfg.txt.style.color = cfg.color(+v);
  };

  const set = (v: number) => {
    cfg.set(v);
    setText(v);
  };

  const sync = (v: number) => {
    cfg.ctrl.value = norm(v) as unknown as string;
  };

  cfg.get().then((v) => {
    const num = +v;
    setText(num);
    sync(num);
  });

  cfg.ctrl.oninput = () => {
    const v = denorm(+cfg.ctrl.value);
    set(v);
  };

  cfg.ctrl.ondblclick = () => {
    sync(1);
    set(1);
  };
}
