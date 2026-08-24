import { defineStore } from "pinia";
import { ref } from "vue";

export type TabType = "query" | "table-data" | "table-design";

export interface WorkspaceTab {
  id: string;
  type: TabType;
  title: string;
  connId: string;
  db?: string;
  table?: string;
  kind?: string; // stable | table | view
}

let seq = 0;

export const useWorkspaceStore = defineStore("workspace", () => {
  const tabs = ref<WorkspaceTab[]>([]);
  const activeId = ref("");

  function setActive(id: string) {
    activeId.value = id;
  }

  function openQueryTab(connId: string, db?: string) {
    seq += 1;
    const id = `query-${Date.now()}-${seq}`;
    const title = `查询 ${seq}`;
    tabs.value.push({ id, type: "query", title, connId, db });
    activeId.value = id;
    return id;
  }

  function openTableTab(
    connId: string,
    db: string,
    table: string,
    kind: string,
    mode: "table-data" | "table-design"
  ) {
    const type = mode;
    const existing = tabs.value.find(
      (t) =>
        t.type === type &&
        t.connId === connId &&
        t.db === db &&
        t.table === table
    );
    if (existing) {
      activeId.value = existing.id;
      return existing.id;
    }
    seq += 1;
    const id = `${type}-${Date.now()}-${seq}`;
    const suffix = kind === "stable" ? " ⚡" : "";
    const title =
      type === "table-data" ? `${table}${suffix}` : `${table} · 设计`;
    tabs.value.push({ id, type, title, connId, db, table, kind });
    activeId.value = id;
    return id;
  }

  function closeTab(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id);
    if (idx < 0) return;
    tabs.value.splice(idx, 1);
    if (activeId.value === id) {
      activeId.value = tabs.value[Math.min(idx, tabs.value.length - 1)]?.id ?? "";
    }
  }

  function closeTabsForConnection(connId: string) {
    tabs.value = tabs.value.filter((t) => t.connId !== connId);
    if (!tabs.value.find((t) => t.id === activeId.value)) {
      activeId.value = tabs.value[0]?.id ?? "";
    }
  }

  return {
    tabs,
    activeId,
    setActive,
    openQueryTab,
    openTableTab,
    closeTab,
    closeTabsForConnection,
  };
});
