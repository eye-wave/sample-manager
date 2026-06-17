import type { SchemaFieldWithValue as Field } from "@typegen/SchemaFieldWithValue";
import { $el } from "../alias";
import { IPC, invoke } from "../invoke/invoke";

type CB<T = unknown> = (key: string, value: T) => void;

/** Creates and immediately binds a single settings field. Returns the wrapper element. */
export function renderField(key: string, field: Field, cb?: CB): HTMLElement {
  const wrap = $el("div");
  wrap.className = "field";

  const label = $el("span");
  label.className = "field-label";
  label.textContent = field.label;
  wrap.appendChild(label);

  const input = buildInput(key, field, cb);
  if (input) wrap.appendChild(input);

  return wrap;
}

// ---------------------------------------------------------------------------
// Field builders — each creates + binds its own element(s)
// ---------------------------------------------------------------------------

function buildInput(key: string, field: Field, cb?: CB): HTMLElement | null {
  switch (field.type) {
    case "string":
      return buildString(key, field, cb);
    case "number":
      return buildNumber(key, field, cb);
    case "bool":
      return buildBool(key, field, cb);
    case "select":
      return buildSelect(key, field, cb);
    case "numberList":
      return buildNumberList(key, field, cb);
    case "stringList":
      return buildStringList(key, field, cb);
    default:
      return null;
  }
}

function onChangeValue(key: string, value: unknown, cb?: CB) {
  if (cb) cb(key, value);
  else {
    const [id, name] = key.split(":");
    if (id && name) invoke(IPC.ConfigurePluginValue, { id, name, data: value });
  }
}

function buildString(key: string, field: Extract<Field, { type: "string" }>, cb?: CB) {
  const input = $el("input") as HTMLInputElement;
  input.type = field.is_password ? "password" : "text";
  input.value = field.value;
  input.oninput = () => onChangeValue(key, input.value, cb);
  return input;
}

function buildNumber(key: string, field: Extract<Field, { type: "number" }>, cb?: CB) {
  const input = $el("input") as HTMLInputElement;
  input.type = "number";
  input.value = String(field.value);
  input.oninput = () => onChangeValue(key, input.value, cb);
  return input;
}

function buildBool(key: string, field: Extract<Field, { type: "bool" }>, cb?: CB) {
  const id = key + performance.now().toString(36);
  const wrap = $el("div");
  wrap.className = "switch";

  const input = $el("input") as HTMLInputElement;
  input.type = "checkbox";
  input.id = id;
  input.checked = !!field.value;
  input.oninput = () => onChangeValue(key, String(input.checked), cb);

  const lbl = $el("label") as HTMLLabelElement;
  lbl.htmlFor = id;

  wrap.appendChild(input);
  wrap.appendChild(lbl);
  return wrap;
}

function buildSelect(key: string, field: Extract<Field, { type: "select" }>, cb?: CB) {
  const select = $el("select") as HTMLSelectElement;

  const appendOptions = (values: string[]) => {
    for (const v of values) {
      const opt = $el("option") as HTMLOptionElement;
      opt.value = v;
      opt.textContent = v;
      opt.selected = v === field.value;
      select.appendChild(opt);
    }
  };

  if ("list" in field.options) {
    appendOptions(field.options.list.values);
  } else if ("grouped" in field.options) {
    for (const [group, values] of Object.entries(field.options.grouped.groups)) {
      const grp = $el("optgroup") as HTMLOptGroupElement;
      grp.label = group;
      appendOptions(values); // appends to select — refactor if nesting matters
      select.appendChild(grp);
    }
  }

  select.oninput = () => onChangeValue(key, select.value, cb);
  return select;
}

function buildNumberList(key: string, field: Extract<Field, { type: "numberList" }>, cb?: CB) {
  const wrap = $el("div");
  wrap.style.display = "contents";

  const inputs: HTMLInputElement[] = Array.from({ length: field.count }, (_, i) => {
    const input = $el("input") as HTMLInputElement;
    input.type = "number";
    input.value = String(field.value.at(i) ?? field.default.at(i) ?? 0);
    wrap.appendChild(input);
    return input;
  });

  const onChange = () =>
    onChangeValue(
      key,
      inputs.map((b) => +b.value),
      cb,
    );
  for (const input of inputs) input.oninput = onChange;

  return wrap;
}

function buildStringList(key: string, field: Extract<Field, { type: "stringList" }>, cb?: CB) {
  const wrap = $el("div");
  const listEl = $el("div");
  const textIn = $el("input") as HTMLInputElement;
  textIn.className = "string-list-input";
  textIn.type = "text";

  const values = new Set<string>(field.value ?? []);

  const onChange = () => {
    const data = [...values];
    onChangeValue(key, data, cb);
  };

  const addItem = (v: string) => {
    const trimmed = v.trim();
    if (!trimmed || values.has(trimmed)) return;
    values.add(trimmed);

    const row = $el("div");
    row.className = "string-list-row";
    const span = $el("span");
    span.className = "string-list-row-text";
    span.textContent = trimmed;
    row.appendChild(span);
    listEl.appendChild(row);
    listEl.scrollTop = listEl.scrollHeight;
    onChange();
  };

  for (const existing of values) addItem(existing);

  textIn.onkeydown = (e) => {
    if (e.key !== "Enter") return;
    e.preventDefault();
    addItem(textIn.value);
    textIn.value = "";
  };

  listEl.onclick = (e) => {
    const row = (e.target as Element).closest(".string-list-row");
    if (!row) return;
    const text = row.querySelector(".string-list-row-text")?.textContent ?? "";
    values.delete(text);
    row.remove();
    onChange();
  };

  wrap.appendChild(textIn);
  wrap.appendChild(listEl);
  return wrap;
}
