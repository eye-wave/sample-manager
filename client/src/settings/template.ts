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

export function renderPluginSettings({ id, config }: PluginInfo) {
  const fields = Object.entries(config).map(([key, data]) => {
    return renderField(`${id}:${key}`, data);
  });

  return fields.join("");
}

function renderField(key: string, data: SchemaFieldWithValue) {
  const { fieldType, value } = data;
  const currentValue = value ?? fieldType.default;

  let inputHtml = "";

  if (fieldType.type === "string") {
    const inputType = fieldType.is_password ? "password" : "text";
    inputHtml = /* HTML */ `<input type="${inputType}" value="${currentValue}" data-key="${key}">`;
  } else if (fieldType.type === "number") {
    inputHtml = /* HTML */ `<input type="number" value="${currentValue}" data-key="${key}">`;
  } else if (fieldType.type === "bool") {
    const id = key + performance.now().toString(36);
    const checked = currentValue ? "checked" : "";

    inputHtml = `<div class=switch><input id=${id} type=checkbox ${checked} data-key=${key}><label for=${id}></label></div>`;
  } else if (fieldType.type === "select") {
    const options = fieldType.options.map((opt) => {
      const selected = opt === currentValue ? "selected" : "";
      return `<option value="${opt}" ${selected}>${opt}</option>`;
    });

    inputHtml = /* HTML */ `<select data-key="${key}">${options.join("")}</select>`;
  }

  return /* HTML */ `
    <div class="field">
      <span class="field-label">${fieldType.label}</span>
      ${inputHtml}
    </div>`;
}

export function bindSettingInputs(el: HTMLElement) {
  const inputs = el.querySelectorAll("[data-key]") as NodeListOf<HTMLInputElement>;

  inputs.forEach((i) => {
    i.oninput = () => {
      const joined = i.dataset.key;
      const [id, name] = joined?.split(":") ?? [];
      if (!id || !name) return;

      const data = i.getAttribute("type") === "checkbox" ? `${i.checked}` : i.value;

      invoke(IPC.CONFIGURE_PLUGIN_VALUE, { id, name, data });
    };
  });
}
