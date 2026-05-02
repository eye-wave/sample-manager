import type { PluginInfo } from "@typegen/PluginInfo";

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

    <button class="btn btn-ghost">Configure</button>
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
