import { defineStore } from "pinia";
import { computed, reactive, ref } from "vue";
import * as api from "@/api";
import type { ConnectionConfig, DatabaseInfo, ServerInfo, TableBrief } from "@/types";

export interface TreeMeta {
  nodeType: "connection" | "database" | "table";
  connId: string;
  db?: string;
  name?: string;
  kind?: string; // stable | table | view
}

export const useConnectionsStore = defineStore("connections", () => {
  const configs = ref<ConnectionConfig[]>([]);
  const serverInfos = reactive<Record<string, ServerInfo>>({});
  const databases = reactive<Record<string, DatabaseInfo[]>>({});
  const tables = reactive<Record<string, TableBrief[]>>({});
  const loading = reactive<Record<string, boolean>>({});
  const expandedKeys = ref<string[]>([]);
  const selectedKeys = ref<string[]>([]);

  const connKey = (id: string) => `conn:${id}`;
  const dbKey = (connId: string, db: string) => `db:${connId}:${db}`;

  async function init() {
    try {
      configs.value = await api.loadConnections();
    } catch (e) {
      console.error("加载连接配置失败", e);
    }
  }

  async function persist() {
    await api.saveConnections(configs.value);
  }

  async function addConfig(config: ConnectionConfig) {
    configs.value.push(config);
    await persist();
  }

  async function updateConfig(config: ConnectionConfig) {
    const idx = configs.value.findIndex((c) => c.id === config.id);
    if (idx >= 0) {
      configs.value[idx] = config;
      await persist();
    }
    // if connected, reconnect with new settings
    if (serverInfos[config.id]) {
      await doDisconnect(config.id);
      await doConnect(config).catch(() => {});
    }
  }

  async function removeConfig(id: string) {
    await doDisconnect(id);
    configs.value = configs.value.filter((c) => c.id !== id);
    await persist();
  }

  function getConfig(id: string): ConnectionConfig | undefined {
    return configs.value.find((c) => c.id === id);
  }

  async function doConnect(config: ConnectionConfig): Promise<ServerInfo> {
    const info = await api.connect(config);
    serverInfos[config.id] = info;
    await loadDatabases(config.id);
    return info;
  }

  async function doDisconnect(id: string) {
    await api.disconnect(id).catch(() => {});
    delete serverInfos[id];
    for (const key of Object.keys(databases)) {
      if (key.startsWith(id + ":") || key === id) delete databases[key];
    }
    for (const key of Object.keys(tables)) {
      if (key.startsWith(id + ":")) delete tables[key];
    }
    expandedKeys.value = expandedKeys.value.filter(
      (k) => !k.startsWith(`conn:${id}:`) && k !== `conn:${id}`
    );
  }

  async function loadDatabases(connId: string) {
    loading[`dbs:${connId}`] = true;
    try {
      databases[connId] = await api.listDatabases(connId);
    } finally {
      delete loading[`dbs:${connId}`];
    }
  }

  async function loadTables(connId: string, db: string) {
    const key = `${connId}:${db}`;
    loading[`tables:${key}`] = true;
    try {
      tables[key] = await api.listTables(connId, db);
    } finally {
      delete loading[`tables:${key}`];
    }
  }

  async function refreshDatabase(connId: string) {
    if (serverInfos[connId]) {
      await loadDatabases(connId);
    }
  }

  async function refreshTables(connId: string, db: string) {
    await loadTables(connId, db);
  }

  return {
    configs,
    serverInfos,
    databases,
    tables,
    loading,
    expandedKeys,
    selectedKeys,
    connKey,
    dbKey,
    init,
    addConfig,
    updateConfig,
    removeConfig,
    getConfig,
    doConnect,
    doDisconnect,
    loadDatabases,
    loadTables,
    refreshDatabase,
    refreshTables,
  };
});
