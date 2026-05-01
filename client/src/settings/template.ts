import type { PluginManifest } from "@typegen/PluginManifest";
import type { SchemaField } from "@typegen/SchemaField";

export function createPluginSettingsSegment(manifest: PluginManifest) {
  const fields = Object.entries(manifest.config_schema);

  return /* HTML */ `
    <p class="section-label">Plugin: ${manifest.name}</p>
    <div class="plugin-config-group" data-plugin-id="${manifest.id}">
      ${fields.map(([id, field]) => renderField(id, field)).join("")}
    </div>
  `;
}

function renderField(id: string, field: SchemaField): string {
  return /* HTML */ `
    <div class="field">
      <div class="field__label">
        <span>${field.label}</span>
      </div>
      <div class="field__control">
        ${renderInput(id, field)}
      </div>
    </div>
  `;
}

function renderInput(id: string, field: SchemaField): string {
  const baseAttr = `id="config-${id}" data-config-key="${id}"`;

  switch (field.type) {
    case "string":
      return /* HTML */ `
        <input type="${field.is_password ? "password" : "text"}"
               ${baseAttr} value="${field.default}">`;

    case "number":
      return /* HTML */ `
        <input type="number" ${baseAttr} value="${field.default}">`;

    case "bool":
      return /* HTML */ `
        <input type="checkbox" ${baseAttr} ${field.default ? "checked" : ""}>`;

    case "select":
      return /* HTML */ `
        <select ${baseAttr}>
          ${field.options
            .map(
              (opt) =>
                `<option value="${opt}" ${opt === field.default ? "selected" : ""}>${opt}</option>`,
            )
            .join("")}
        </select>`;

    default:
      return "";
  }
}
