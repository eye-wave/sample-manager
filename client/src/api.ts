import type { SampleEntry } from "@typegen/SampleEntry";
import type { SearchRequest } from "@typegen/SearchRequest";
import type { PluginSidebarView } from "@typegen/PluginSidebarView";
import type { SchemaFieldWithValue } from "@typegen/SchemaFieldWithValue";

import { invoke, IPC } from "./invoke/invoke";

export const getSampleMetedata = async (path: string): Promise<SampleEntry> =>
  JSON.parse(await invoke(IPC.GET_SAMPLE_METADATA, path));

export const toggleFav = (path: string) => invoke(IPC.TOGGLE_SAMPLE_FAV, path);
export const isFav = async (path: string) => !!+(await invoke(IPC.IS_SAMPLE_FAV, path));

export const callSampleSearch = async (params: SearchRequest) =>
  invoke(IPC.SEARCH_FOR_SAMPLE, params);

export const getPluginPaths = (): Promise<PluginSidebarView[]> =>
  invoke(IPC.GET_PLUGIN_PATHS).then((p) => JSON.parse(p));
export const getSampleFolders = () =>
  invoke(IPC.GET_SAMPLE_FOLDERS).then((res) => res.split("\n").filter((e) => e));

export const getConfigFields = (): Promise<Record<string, SchemaFieldWithValue>> =>
  invoke(IPC.GET_CONFIG_FIELDS).then(JSON.parse);
