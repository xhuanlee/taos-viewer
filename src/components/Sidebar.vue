<script setup lang="ts">
import { computed, h, nextTick, ref, type VNodeChild } from "vue";
import {
  NButton,
  NDropdown,
  NEmpty,
  NInput,
  NTree,
  NTooltip,
  useDialog,
  useMessage,
  type DropdownOption,
  type TreeOption,
} from "naive-ui";
import {
  IconDatabase,
  IconGrid,
  IconPencil,
  IconPlay,
  IconPlus,
  IconRefresh,
  IconServer,
  IconStable,
  IconTable,
  IconTerminal,
  IconTrash,
  IconView,
  IconX,
} from "@/components/icons";
import { useConnectionsStore } from "@/stores/connections";
import { useWorkspaceStore } from "@/stores/workspace";
import { executeBatch, showCreateTable } from "@/api";
import { quoteIdent } from "@/utils/sql";
import type { ConnectionConfig } from "@/types";

const emit = defineEmits<{
  (e: "new-connection"): void;
  (e: "edit-connection", config: ConnectionConfig): void;
}>();

const connStore = useConnectionsStore();
const workspace = useWorkspaceStore();
const message = useMessage();
const dialog = useDialog();

// ---------- search ----------

const search = ref("");

function filterTreeNode(pattern: string, node: TreeOption): boolean {
  const label = String(node.label ?? "");
  return label.toLowerCase().includes(pattern.toLowerCase());
}

// ---------- tree data ----------

interface TreeExtra extends TreeOption {
  nodeType: "connection" | "database" | "table";
  connId: string;
  db?: string;
  name?: string;
  kind?: string;
}

const STABLE_COLOR = "#38bdf8";
const TABLE_COLOR = "#34d399";
const VIEW_COLOR = "#a78bfa";
const DB_COLOR = "#eab308";

function withColor(icon: ReturnType<typeof IconServer>, color: string, dim = false) {
  return () =>
    h(icon, {
      size: 15,
      style: { color: dim ? "#6b7280" : color },
    });
}

const treeData = computed<TreeOption[]>(() => {
  return connStore.configs.map((c) => {
    const connected = !!connStore.serverInfos[c.id];
    const dbs = connStore.databases[c.id] ?? [];
    const children: TreeOption[] = dbs.map((db) => {
      const key = connStore.dbKey(c.id, db.name);
      const tbs = connStore.tables[`${c.id}:${db.name}`] ?? [];
      const tbNodes: TreeOption[] = tbs.map((tb) => ({
        key: `t:${c.id}:${db.name}:${tb.name}`,
        label: tb.name,
        isLeaf: true,
        prefix:
          tb.kind === "stable"
            ? withColor(IconStable, STABLE_COLOR)
            : tb.kind === "view"
              ? withColor(IconView, VIEW_COLOR)
              : withColor(IconTable, TABLE_COLOR),
        nodeType: "table",
        connId: c.id,
        db: db.name,
        name: tb.name,
        kind: tb.kind,
      }));
      return {
        key,
        label: db.name,
        prefix: withColor(IconDatabase, DB_COLOR),
        children: tbNodes,
        nodeType: "database",
        connId: c.id,
        db: db.name,
      };
    });
    return {
      key: connStore.connKey(c.id),
      label: c.name,
      prefix: withColor(IconServer, connected ? TABLE_COLOR : "#6b7280"),
      suffix: () =>
        connected
          ? h(
              "span",
              { class: "conn-version", title: `TDengine ${connStore.serverInfos[c.id].version}` },
              `v${connStore.serverInfos[c.id].version}`
            )
          : h("span", { class: "conn-offline" }, "离线"),
      children,
      nodeType: "connection",
      connId: c.id,
      name: c.name,
    };
  });
});

// ---------- expansion / loading ----------

const treeLoadingKeys = computed(() =>
  Object.entries(connStore.loading)
    .filter(([, v]) => v)
    .map(([k]) => {
      if (k.startsWith("dbs:")) return connStore.connKey(k.slice(4));
      if (k.startsWith("tables:")) {
        const rest = k.slice(7);
        const sep = rest.indexOf(":");
        return connStore.dbKey(rest.slice(0, sep), rest.slice(sep + 1));
      }
      return "";
    })
    .filter(Boolean)
);

async function onExpand(keys: string[]) {
  connStore.expandedKeys = keys;
  for (const key of keys) {
    if (key.startsWith("conn:")) {
      const connId = key.slice(5);
      if (!connStore.serverInfos[connId]) {
        const config = connStore.getConfig(connId);
        if (!config) continue;
        try {
          await connStore.doConnect(config);
          message.success(`已连接到 ${config.name}`);
        } catch (e) {
          message.error(`连接失败: ${e}`);
        }
      } else if (!connStore.databases[connId]) {
        connStore.loadDatabases(connId).catch((e) => message.error(String(e)));
      }
    } else if (key.startsWith("db:")) {
      const rest = key.slice(3);
      const sep = rest.indexOf(":");
      const connId = rest.slice(0, sep);
      const db = rest.slice(sep + 1);
      if (!connStore.tables[`${connId}:${db}`]) {
        connStore.loadTables(connId, db).catch((e) => message.error(String(e)));
      }
    }
  }
}

// ---------- node interactions ----------

function parseExtra(option: TreeOption): TreeExtra {
  return option as unknown as TreeExtra;
}

function onNodeProps({ option }: { option: TreeOption }) {
  return {
    onContextmenu: (e: MouseEvent) => {
      e.preventDefault();
      showDropdown.value = false;
      nextTick().then(() => {
        dropdownNode.value = option;
        dropdownX.value = e.clientX;
        dropdownY.value = e.clientY;
        showDropdown.value = true;
      });
    },
    onDblclick: () => {
      const extra = parseExtra(option);
      if (extra.nodeType === "connection") {
        if (!connStore.serverInfos[extra.connId]) {
          onExpand([...connStore.expandedKeys, option.key as string]);
        }
      } else if (extra.nodeType === "table" && extra.db && extra.name) {
        workspace.openTableTab(extra.connId, extra.db, extra.name, extra.kind ?? "table", "table-data");
      }
    },
  };
}

// ---------- context menu ----------

const showDropdown = ref(false);
const dropdownX = ref(0);
const dropdownY = ref(0);
const dropdownNode = ref<TreeOption | null>(null);

const dropdownOptions = computed<DropdownOption[]>(() => {
  const node = dropdownNode.value;
  if (!node) return [];
  const extra = parseExtra(node);
  const isMac = navigator.platform.toUpperCase().includes("MAC");
  const mod = isMac ? "⌘" : "Ctrl+";

  if (extra.nodeType === "connection") {
    const connected = !!connStore.serverInfos[extra.connId];
    return [
      {
        label: connected ? "断开连接" : "连接",
        key: "toggle-conn",
        icon: () => h(IconPlay, { size: 14 }),
      },
      {
        label: "新建查询",
        key: "new-query",
        icon: () => h(IconTerminal, { size: 14 }),
        disabled: !connected,
      },
      { type: "divider", key: "d1" },
      {
        label: "编辑连接",
        key: "edit-conn",
        icon: () => h(IconPencil, { size: 14 }),
      },
      {
        label: "删除连接",
        key: "del-conn",
        icon: () => h(IconTrash, { size: 14 }),
      },
    ];
  }
  if (extra.nodeType === "database" && extra.db) {
    return [
      {
        label: `新建查询（${mod}Enter 执行）`,
        key: "new-query",
        icon: () => h(IconTerminal, { size: 14 }),
      },
      {
        label: "刷新表列表",
        key: "refresh-tables",
        icon: () => h(IconRefresh, { size: 14 }),
      },
      { type: "divider", key: "d1" },
      {
        label: "删除数据库",
        key: "drop-db",
        icon: () => h(IconTrash, { size: 14 }),
      },
    ];
  }
  // table
  return [
    {
      label: "浏览数据",
      key: "view-data",
      icon: () => h(IconGrid, { size: 14 }),
    },
    {
      label: "设计表（字段维护）",
      key: "design-table",
      icon: () => h(IconTable, { size: 14 }),
    },
    {
      label: "复制建表语句",
      key: "copy-ddl",
      icon: () => h(IconDatabase, { size: 14 }),
    },
    { type: "divider", key: "d1" },
    {
      label: "刷新",
      key: "refresh-tables",
      icon: () => h(IconRefresh, { size: 14 }),
    },
    {
      label: "删除表",
      key: "drop-table",
      icon: () => h(IconTrash, { size: 14 }),
    },
  ];
});

async function onDropdownSelect(key: string) {
  showDropdown.value = false;
  const node = dropdownNode.value;
  if (!node) return;
  const extra = parseExtra(node);

  try {
    if (key === "toggle-conn") {
      if (connStore.serverInfos[extra.connId]) {
        await connStore.doDisconnect(extra.connId);
        workspace.closeTabsForConnection(extra.connId);
        message.success("已断开连接");
      } else {
        const config = connStore.getConfig(extra.connId);
        if (!config) return;
        const info = await connStore.doConnect(config);
        message.success(`已连接：TDengine v${info.version}`);
      }
    } else if (key === "new-query") {
      workspace.openQueryTab(extra.connId, extra.db);
    } else if (key === "edit-conn") {
      const config = connStore.getConfig(extra.connId);
      if (config) emit("edit-connection", config);
    } else if (key === "del-conn") {
      const config = connStore.getConfig(extra.connId);
      if (!config) return;
      dialog.warning({
        title: "删除连接",
        content: `确定删除连接 “${config.name}” 吗？该操作仅删除本地配置，不会影响服务器。`,
        positiveText: "删除",
        negativeText: "取消",
        onPositiveClick: async () => {
          await connStore.removeConfig(extra.connId);
          workspace.closeTabsForConnection(extra.connId);
          message.success("连接已删除");
        },
      });
    } else if (key === "refresh-tables") {
      if (extra.nodeType === "database" && extra.db) {
        await connStore.refreshTables(extra.connId, extra.db);
        message.success("已刷新");
      }
    } else if (key === "drop-db") {
      if (!extra.db) return;
      dialog.error({
        title: "删除数据库",
        content: `确定删除数据库 “${extra.db}” 吗？其中所有数据将被永久删除，不可恢复！`,
        positiveText: "删除数据库",
        negativeText: "取消",
        onPositiveClick: async () => {
          await executeBatch({
            connId: extra.connId,
            sqls: [`DROP DATABASE ${quoteIdent(extra.db!)}`],
          });
          await connStore.refreshDatabase(extra.connId);
          message.success(`数据库 ${extra.db} 已删除`);
        },
      });
    } else if (key === "view-data") {
      workspace.openTableTab(
        extra.connId,
        extra.db!,
        extra.name!,
        extra.kind ?? "table",
        "table-data"
      );
    } else if (key === "design-table") {
      workspace.openTableTab(
        extra.connId,
        extra.db!,
        extra.name!,
        extra.kind ?? "table",
        "table-design"
      );
    } else if (key === "copy-ddl") {
      const ddl = await showCreateTable(
        extra.connId,
        extra.db!,
        extra.name!,
        extra.kind ?? "table"
      );
      await navigator.clipboard.writeText(ddl);
      message.success("建表语句已复制到剪贴板");
    } else if (key === "drop-table") {
      const obj = extra.kind === "stable" ? "STABLE" : "TABLE";
      dialog.error({
        title: `删除${extra.kind === "stable" ? "超级表" : "表"}`,
        content: `确定删除 “${extra.name}” 吗？其中所有数据将被永久删除！`,
        positiveText: "删除",
        negativeText: "取消",
        onPositiveClick: async () => {
          await executeBatch({
            connId: extra.connId,
            db: extra.db,
            sqls: [`DROP ${obj} ${quoteIdent(extra.db!)}.${quoteIdent(extra.name!)}`],
          });
          await connStore.refreshTables(extra.connId, extra.db!);
          message.success(`表 ${extra.name} 已删除`);
        },
      });
    }
  } catch (e) {
    message.error(String(e));
  }
}

const nodePropsFn = onNodeProps;

function renderSuffix(): VNodeChild | undefined {
  return undefined;
}
</script>

<template>
  <div class="sidebar">
    <div class="sidebar-header">
      <span class="sidebar-title">连接</span>
      <n-tooltip placement="bottom">
        <template #trigger>
          <n-button size="tiny" quaternary circle @click="emit('new-connection')">
            <template #icon>
              <IconPlus :size="14" />
            </template>
          </n-button>
        </template>
        新建连接
      </n-tooltip>
    </div>

    <div v-if="connStore.configs.length > 0" class="sidebar-search">
      <n-input
        v-model:value="search"
        size="small"
        placeholder="搜索…"
        clearable
      />
    </div>

    <div class="sidebar-tree">
      <n-tree
        v-if="connStore.configs.length > 0"
        :data="treeData"
        :pattern="search || undefined"
        :filter="filterTreeNode"
        block-line
        :expanded-keys="connStore.expandedKeys"
        :selected-keys="connStore.selectedKeys"
        :loading-keys="treeLoadingKeys"
        :node-props="nodePropsFn"
        :render-suffix="renderSuffix"
        :on-update:expanded-keys="onExpand"
        :on-update:selected-keys="(keys: string[]) => (connStore.selectedKeys = keys)"
        :virtual-scroll="true"
      />
      <div v-else class="sidebar-empty">
        <n-empty description="还没有连接">
          <template #extra>
            <n-button size="small" type="primary" @click="emit('new-connection')">
              <template #icon>
                <IconPlus :size="14" />
              </template>
              新建连接
            </n-button>
          </template>
        </n-empty>
      </div>
    </div>

    <n-dropdown
      trigger="manual"
      placement="bottom-start"
      :show="showDropdown"
      :x="dropdownX"
      :y="dropdownY"
      :options="dropdownOptions"
      @select="onDropdownSelect"
      @clickoutside="showDropdown = false"
    />
  </div>
</template>

<style scoped>
.sidebar {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--app-border);
  background: var(--app-card);
  min-height: 0;
  min-width: 0;
}

.sidebar-header {
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 10px 0 14px;
}

.sidebar-title {
  font-size: 12px;
  font-weight: 600;
  opacity: 0.75;
  letter-spacing: 0.4px;
}

.sidebar-search {
  padding: 0 10px 6px;
}

.sidebar-tree {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 2px 4px 12px;
}

.sidebar-empty {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

:deep(.conn-version) {
  font-size: 11px;
  color: #34d399;
  opacity: 0.85;
  font-family: "SF Mono", Menlo, Consolas, monospace;
}

:deep(.conn-offline) {
  font-size: 11px;
  opacity: 0.45;
}

:deep(.n-tree-node-content__prefix) {
  display: inline-flex;
}

:deep(.n-tree .n-tree-node) {
  border-radius: 6px;
}
</style>
