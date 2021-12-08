import http from "@/http-common";
import {
  BasicResponse,
  DEFAULT_STORAGE,
  DEFAULT_STORAGE_LIST,
  Storage,
  StorageList,
} from "../Response";

export async function getStorages(token: string) {
  const value = await http.get("/api/storages/list", {
    headers: {
      Authorization: "Bearer " + token,
    },
  });

  if (value.status != 200) {
    return DEFAULT_STORAGE_LIST;
  }
  const data = value.data as BasicResponse<unknown>;
  if (data.success) {
    return data.data as StorageList;
  }

  return DEFAULT_STORAGE_LIST;
}

export async function getStoragesPublicAccess() {
  const value = await http.get("/storages.json", {});

  if (value.status != 200) {
    return [];
  }
  const data = value.data as BasicResponse<unknown>;
  if (data.success) {
    return data.data as Array<string>;
  }

  return [];
}
export async function getStorage(token: string, id: number) {
  const value = await http.get("/api/storages/id/" + id, {
    headers: {
      Authorization: "Bearer " + token,
    },
  });

  if (value.status != 200) {
    return DEFAULT_STORAGE;
  }
  const data = value.data as BasicResponse<unknown>;
  if (data.success) {
    return data.data as Storage;
  }

  return DEFAULT_STORAGE;
}
