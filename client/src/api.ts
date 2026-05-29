import type { SampleEntry } from "@typegen/SampleEntry";
import type { SearchRequest } from "@typegen/SearchRequest";

import { invoke, IPC } from "./invoke/invoke";

export const getSampleMetedata = async (path: string): Promise<SampleEntry> =>
  JSON.parse(await invoke(IPC.GET_SAMPLE_METADATA, path));

export const toggleFav = (path: string) => invoke(IPC.TOGGLE_SAMPLE_FAV, path);
export const isFav = async (path: string) => !!+(await invoke(IPC.IS_SAMPLE_FAV, path));

export const callSampleSearch = async (params: SearchRequest) =>
  invoke(IPC.SEARCH_FOR_SAMPLE, params);
