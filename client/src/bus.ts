import type { BusEvent } from "./bus/events";

export type { BusEvent } from "./bus/events";
export * as BUS from "./bus/events";

type Handler<T> = (payload: T) => void;

type Slot = {
  event: BusEvent;
  handler: Handler<unknown>;
};

const bus: Slot[] = [];

export function emit<T>(event: BusEvent, payload?: T) {
  for (let i = 0; i < bus.length; i++) {
    const slot = bus[i];

    if (slot.event === event) {
      slot.handler(payload);
    }
  }
}

export function on(event: BusEvent, handler: Handler<unknown>) {
  const slot: Slot = { event, handler };

  bus.push(slot);

  return () => {
    const index = bus.indexOf(slot);

    if (index !== -1) {
      bus.splice(index, 1);
    }
  };
}
