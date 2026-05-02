import type { PluginInfo } from "@typegen/PluginInfo";
import type { SchemaFieldWithValue } from "@typegen/SchemaFieldWithValue";

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

export function renderField(key: string, data: SchemaFieldWithValue) {
  const { fieldType, value } = data;
  const currentValue = value ?? fieldType.default;
  const id = `plugin_setting_${key.replace(/\s+/g, "_")}`;

  let inputHtml = "";

  switch (fieldType.type) {
    case "string":
      inputHtml = /* HTML */ `
        <input
          type="${fieldType.is_password ? "password" : "text"}"
          id="${id}"
          value="${currentValue}"
          data-key="${key}"
        >`;
      break;

    case "number":
      inputHtml = /* HTML */ `
        <input
          type="number"
          id="${id}"
          value="${currentValue}"
          data-key="${key}"
        >`;
      break;

    case "bool":
      inputHtml = /* HTML */ `
        <input
          type="checkbox"
          id="${id}"
          ${currentValue ? "checked" : ""}
          data-key="${key}"
        >`;
      break;

    case "select":
      inputHtml = /* HTML */ `
        <select id="${id}" data-key="${key}">
          ${fieldType.options
            .map(
              (opt) => /* HTML */ `
              <option value="${opt}" ${opt === currentValue ? "selected" : ""}>
                ${opt}
              </option>`,
            )
            .join("")}
        </select>`;
      break;
  }

  return /* HTML */ `
    <div class="field">
      <span class="field-label">${fieldType.label}</span>
      ${inputHtml}
    </div>`;
}
