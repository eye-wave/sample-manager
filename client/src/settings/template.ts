import type { PluginInfo } from "@typegen/PluginInfo";
import type { SchemaFieldWithValue } from "@typegen/SchemaFieldWithValue";
import * as IPC from "../gen/ipc-gen";
import { invoke } from "../invoke/invoke";

export function createPluginCard(info: PluginInfo) {
  return /* HTML */ `<div class="card">
    ${info.icon ? cardIcon(info.icon) : ""}

    <div class="card-header">
      <h2 class="card-name">${info.name}</h2>
      <span class="card-version">${info.version}</span>
    </div>

    <p class="card-description">${info.description}</p>

    <div class="card-capabilities">
      ${capability(info.capabilities.encrypted_storage && "Safe storage")}
      ${capability(info.capabilities.network && "Network")}
      ${capability(info.capabilities.storage && "Storage")}
    </div>

    ${info.capabilities.network ? hosts(info.capabilities.network_allowlist) : ""}

    <button data-id="${info.id}" class="btn btn-ghost">Configure</button>
  </div>`;
}

const cardIcon = (icon: string) => /* HTML */ `<div class="card-icon">${icon}</div>`;
const capability = (name: string | false) =>
  name ? /* HTML */ `<span class="card-tag">${name}</span>` : "";

const hosts = (hosts: string[]) => {
  return /* HTML */ `<details class="card-hosts">
    <summary>Allowed Hosts (${hosts.length})</summary>
    <ul>${hosts.map(host).join("")}</ul>
  </details>`;
};

const host = (label: string) => `<li>https://${label}/*</li>`;

export function renderSettings({
  id,
  config,
}: {
  id: string;
  config: Record<string, SchemaFieldWithValue>;
}) {
  const fields = Object.entries(config)
    .toSorted((a, b) => a[0].localeCompare(b[0]))
    .map(([key, data]) => {
      return renderField(`${id}:${key}`, data);
    });

  return fields.join("");
}

function renderField(key: string, field: SchemaFieldWithValue) {
  const currentValue = field.value;

  let inputHtml = "";

  if (field.type === "string") {
    const inputType = field.is_password ? "password" : "text";
    inputHtml = /* HTML */ `<input type="${inputType}" value="${currentValue}" data-key="${key}">`;
  } else if (field.type === "number") {
    inputHtml = /* HTML */ `<input type="number" value="${currentValue}" data-key="${key}">`;
  } else if (field.type === "bool") {
    const id = key + performance.now().toString(36);
    const checked = currentValue ? "checked" : "";

    inputHtml = `<div class=switch><input id=${id} type=checkbox ${checked} data-key=${key}><label for=${id}></label></div>`;
  } else if (field.type === "select") {
    let optionsHtml = "";

    if ("list" in field.options) {
      optionsHtml = field.options.list.values
        .map((opt) => {
          const selected = opt === currentValue ? "selected" : "";
          return `<option value="${opt}" ${selected}>${opt}</option>`;
        })
        .join("");
    } else if ("grouped" in field.options) {
      optionsHtml = Object.entries(field.options.grouped.groups)
        .map(([group, opts]) => {
          const optHtml = opts
            .map((opt) => {
              const selected = opt === currentValue ? "selected" : "";
              return `<option value="${opt}" ${selected}>${opt}</option>`;
            })
            .join("");

          return `<optgroup label="${group}">${optHtml}</optgroup>`;
        })
        .join("");
    }

    inputHtml = `<select data-key="${key}">${optionsHtml}</select>`;
  } else if (field.type === "numberList") {
    inputHtml = `<div style="display:contents" data-key="${key}">`;

    for (let i = 0; i < field.count; i++) {
      const val = field.value.at(i) ?? field.default.at(i) ?? 0;

      inputHtml += /* HTML */ `<input type="number" value="${val}">`;
    }

    inputHtml += `</div>`;
  }

  return /* HTML */ `
    <div class="field">
      <span class="field-label">${field.label}</span>
      ${inputHtml}
    </div>`;
}

export function bindSettingInputs(el: HTMLElement, cb?: (key: string, value: string) => void) {
  const inputs = el.querySelectorAll("[data-key]") as NodeListOf<HTMLInputElement>;

  inputs.forEach((i) => {
    const [id, name] = i.dataset.key?.split(":") ?? [];
    if (!id || !name) return;

    if (i.tagName === "DIV") {
      const subInputs = Array.from(i.querySelectorAll("input"));

      subInputs.forEach((s) => {
        s.oninput = () => {
          const data = JSON.stringify(subInputs.map((b) => +b.value));
          if (typeof cb === "undefined")
            invoke(IPC.CONFIGURE_PLUGIN_VALUE, { id, name, data });
          else cb(name, data);
        };
      });

      return;
    }

    i.oninput = () => {
      const data = i.getAttribute("type") === "checkbox" ? `${i.checked}` : i.value;

      if (typeof cb === "undefined") invoke(IPC.CONFIGURE_PLUGIN_VALUE, { id, name, data });
      else cb(name, data);
    };
  });
}
