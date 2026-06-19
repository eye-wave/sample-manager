type Subscriber<T> = (value: T) => void;
type Unsubscribe = () => void;
type Eq<T, U extends T = T> = (a: T, b: U) => boolean;

export const shallowEq = <T extends object>(a: T, b: T) =>
  (Object.keys(a) as (keyof T)[]).every((k) => a[k] === b[k]);

export const arrayEq = <T, A extends Array<T>>(a: A, b: A) =>
  a.length === b.length && a.every((v, i) => v === b[i]);

// --- Writable ---

export type Writable<T> = {
  eq?: Eq<T>;
  get: () => T;
  set: (value: T) => void;
  update: (fn: (value: T) => T) => void;
  subscribe: (cb: Subscriber<T>) => Unsubscribe;
  ping: () => void;
};

export type Readable<T> = {
  get: () => T;
  subscribe: (cb: Subscriber<T>) => Unsubscribe;
  ping: () => void;
};

export function writable<T>(initial: T, eq: Eq<T> = Object.is): Writable<T> {
  let value = initial;
  const subs = new Set<Subscriber<T>>();

  const notify = () => {
    for (const cb of subs) cb(value);
  };

  const set = (next: T) => {
    if (eq(value, next)) return;
    value = next;
    notify();
  };

  return {
    eq,
    set,
    get: () => value,
    update(fn) {
      set(fn(value));
    },
    subscribe(cb) {
      subs.add(cb);
      return () => subs.delete(cb);
    },
    ping: notify,
  };
}

// --- Derived ---

export function derived<T, U>(
  store: Readable<T>,
  fn: (value: T) => U,
  eq: Eq<U> = Object.is,
): Readable<U> {
  let value = fn(store.get());
  const subs = new Set<Subscriber<U>>();

  const notify = () => {
    for (const cb of subs) cb(value);
  };

  store.subscribe((v) => {
    const next = fn(v);

    if (eq(value, next)) return;

    value = next;
    notify();
  });

  return {
    get: () => value,

    subscribe(cb) {
      subs.add(cb);
      return () => subs.delete(cb);
    },

    ping() {
      notify();
    },
  };
}

export type Event<T = void> = {
  emit: (payload: T) => void;
  on: (cb: (payload: T) => void) => Unsubscribe;
  off: (cb: (payload: T) => void) => void;
};

export const event = <T = void>(): Event<T> => {
  const subs = new Set<(payload: T) => void>();

  return {
    emit(payload) {
      for (const cb of subs) cb(payload);
    },

    on(cb) {
      subs.add(cb);
      return () => subs.delete(cb);
    },

    off(cb) {
      subs.delete(cb);
    },
  };
};
