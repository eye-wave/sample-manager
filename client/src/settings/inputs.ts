import type { SchemaFieldWithValue as Field } from "@typegen/SchemaFieldWithValue";
import { invoke, IPC } from "../invoke/invoke";

export function renderField(key: string, field: Field) {
  return /* HTML */ `
    <div class="field">
      <span class="field-label">${field.label}</span>
      ${fields[field.type]?.(key, field) ?? ""}
    </div>`;
}

const fields = {
  string: renderStringInput,
  number: renderNumberInput,
  bool: renderBoolInput,
  select: renderSelectInput,
  numberList: renderNumberListInput,
  stringList: renderStringListInput,
} as Record<string, (key: string, field: Field) => string>;

function renderStringInput(key: string, field: Extract<Field, { type: "string" }>) {
  const inputType = field.is_password ? "password" : "text";

  return /* HTML */ `<input
      type="${inputType}"
      value="${field.value}"
      data-field="string"
      data-key="${key}"
    >`;
}

function renderNumberInput(key: string, field: Extract<Field, { type: "number" }>) {
  return /* HTML */ `<input
      type="number"
      data-field="number"
      value="${field.value}"
      data-key="${key}"
    >`;
}

function renderBoolInput(key: string, field: Extract<Field, { type: "bool" }>) {
  const id = key + performance.now().toString(36);
  const checked = field.value ? "checked" : "";

  return /* HTML */ `<div class="switch">
      <input
        id="${id}"
        type="checkbox"
        ${checked}
        data-key="${key}"
        data-field="bool"
      >
      <label for="${id}"></label>
    </div>`;
}

function renderSelectInput(key: string, field: Extract<Field, { type: "select" }>) {
  return /* HTML */ `<select data-key="${key}" data-field="select">${renderSelectOptions(field)}</select>`;
}

function renderSelectOptions(field: Extract<Field, { type: "select" }>) {
  const renderOption = (value: string, currentValue: string) => {
    const selected = value === currentValue ? " selected" : "";

    return `<option value="${value}"${selected}>${value}</option>`;
  };

  if ("list" in field.options) {
    return field.options.list.values.map((opt) => renderOption(opt, field.value)).join("");
  }

  if ("grouped" in field.options) {
    return Object.entries(field.options.grouped.groups)
      .map(([group, opts]) => {
        const options = opts.map((opt) => renderOption(opt, field.value)).join("");

        return `<optgroup label="${group}">${options}</optgroup>`;
      })
      .join("");
  }

  return "";
}

function renderNumberListInput(key: string, field: Extract<Field, { type: "numberList" }>) {
  const inputs = Array.from({ length: field.count }, (_, i) => {
    const val = field.value.at(i) ?? field.default.at(i) ?? 0;

    return /* HTML */ `<input type="number" value="${val}">`;
  }).join("");

  return /* HTML */ `<div style="display:contents" data-key="${key}" data-field="numberList">
      ${inputs}
    </div>`;
}

function renderStringListInput(
  key: string,
  field: Extract<Field, { type: "stringList" }>,
): string {
  const rows = (field.value ?? [])
    .map(
      (item) => /* HTML */ `<div class="string-list-row" data-value="${item}">
        <span class="string-list-row-text">${item}</span>
      </div>`,
    )
    .join("");

  return /* HTML */ `<div class="field" data-key="${key}" data-field="stringList">
    <div>
      <input
        class="string-list-input"
        type="text"
        data-key="${key}"
        data-field="stringList"
      />
      <div class="string-list-rows">${rows}</div>
    </div>
  </div>`;
}

type InputCallback<T> = (key: string, value: T) => void;

export function bindSettingInputs(el: HTMLElement, cb?: InputCallback<unknown>) {
  const inputs = el.querySelectorAll("[data-key]") as NodeListOf<HTMLInputElement>;

  inputs.forEach((i) => {
    const [id, name] = i.dataset.key?.split(":") ?? [];
    const field = i.dataset.field;
    if (!id || !name || !field) return;

    if (field === "numberList") bindNumbersInput(i, id, name, cb);
    else if (field === "stringList") bindListInput(i, id, name, cb);
    else {
      i.oninput = () => {
        const data = i.getAttribute("type") === "checkbox" ? `${i.checked}` : i.value;

        if (typeof cb === "undefined") invoke(IPC.CONFIGURE_PLUGIN_VALUE, { id, name, data });
        else cb(name, data);
      };
    }
  });
}

function bindListInput(
  i: HTMLDivElement,
  id: string,
  name: string,
  cb?: InputCallback<string[]>,
) {
  const listEl = i.querySelector(".string-list-rows") as HTMLDivElement;
  const input = i.querySelector(".string-list-input") as HTMLInputElement;

  const values = new Set<string>();
  const getValues = () => [...values];

  const onChange = () => {
    const data = getValues();
    if (typeof cb === "undefined") invoke(IPC.CONFIGURE_PLUGIN_VALUE, { id, name, data });
    else cb(name, data);
  };

  const addItem = (value: string) => {
    const trimmed = value.trim();
    if (!trimmed || values.has(trimmed)) return;

    values.add(trimmed);

    const row = document.createElement("div");
    row.className = "string-list-row";
    row.setAttribute("data-value", trimmed);
    row.innerHTML = `<span class="string-list-row-text">${trimmed}</span>`;
    listEl.appendChild(row);
    listEl.scrollTop = listEl.scrollHeight;

    onChange();
  };

  input.onkeydown = (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      addItem(input.value);
      input.value = "";
    }
  };

  listEl.onclick = (e: MouseEvent) => {
    const row = (e.target as Element).closest(".string-list-row");
    row?.remove();

    onChange();
  };
}

function bindNumbersInput(
  i: HTMLDivElement,
  id: string,
  name: string,
  cb?: InputCallback<number[]>,
) {
  const subInputs = Array.from(i.querySelectorAll("input"));

  subInputs.forEach((s) => {
    s.oninput = () => {
      const data = subInputs.map((b) => +b.value);

      if (typeof cb === "undefined")
        invoke(IPC.CONFIGURE_PLUGIN_VALUE, { id, name, data: JSON.stringify(data) });
      else cb(name, data);
    };
  });
}
