import type { DownloadRequest } from "@typegen/DownloadRequest";
import type { PluginSidebarView } from "@typegen/PluginSidebarView";
import type { SampleEntry } from "@typegen/SampleEntry";
import type { SchemaFieldWithValue } from "@typegen/SchemaFieldWithValue";
import type { SearchRequest } from "@typegen/SearchRequest";

import { invoke, IPC } from "./invoke/invoke";

const parse = JSON.parse;

export const getSampleMetedata = async (path: string): Promise<SampleEntry> =>
  parse(await invoke(IPC.GetSampleMetadata, path));

export const toggleFav = (path: string) => invoke(IPC.ToggleSampleFav, path);
export const isFav = async (path: string) => !!+(await invoke(IPC.IsSampleFav, path));

export const callSampleSearch = async (params: SearchRequest) =>
  invoke(IPC.SearchForSample, params);
export const callOnlineSearch = async (params: SearchRequest) =>
  invoke(IPC.PluginSearchForSample, params);

export const getPluginPaths = (): Promise<PluginSidebarView[]> =>
  invoke(IPC.GetPluginPaths).then(parse);
export const getSampleFolders = () =>
  invoke(IPC.GetSampleFolders).then((res) => res.split("\n").filter((e) => e));

export const getConfigFields = (): Promise<Record<string, SchemaFieldWithValue>> =>
  invoke(IPC.GetConfigField).then(parse);

export const downloadFile = (params: DownloadRequest) =>
  invoke(IPC.PluginDownloadFile, params);
