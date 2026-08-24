<script setup lang="ts">
import { h } from "vue";
import { NButton, NTabs, NTabPane, type TabsProps } from "naive-ui";
import QueryTab from "@/components/QueryTab.vue";
import TableDataTab from "@/components/TableDataTab.vue";
import TableDesignTab from "@/components/TableDesignTab.vue";
import {
  IconBolt,
  IconGrid,
  IconTable,
  IconTerminal,
} from "@/components/icons";
import { useConnectionsStore } from "@/stores/connections";
import { useWorkspaceStore, type WorkspaceTab } from "@/stores/workspace";
import { useMessage } from "naive-ui";

const emit = defineEmits<{
  (e: "new-connection"): void;
}>();

const workspace = useWorkspaceStore();
const connStore = useConnectionsStore();
const message = useMessage();

function tabIcon(tab: WorkspaceTab) {
  if (tab.type === "query") return IconTerminal;
  if (tab.type === "table-data") return IconGrid;
  return IconTable;
}

function renderTabLabel(tab: WorkspaceTab) {
  return () =>
    h("span", { class: "tab-label" }, [
      h(tabIcon(tab), { size: 13, style: { marginRight: "5px", opacity: 0.8 } }),
      tab.title,
    ]);
}

function onClose(name: string | number) {
  workspace.closeTab(String(name));
}

function onTabsUpdate(value: string | number) {
  workspace.setActive(String(value));
}

const tabsProps: TabsProps = {
  type: "card",
  size: "small",
  closable: true,
  animated: false,
};

function newQuery() {
  const connected = connStore.configs.filter((c) => connStore.serverInfos[c.id]);
  if (connected.length === 0) {
    message.warning("请先在左侧建立连接");
    emit("new-connection");
    return;
  }
  workspace.openQueryTab(connected[0].id, connected[0].database ?? undefined);
}
</script>

<template>
  <div class="workspace">
    <n-tabs
      v-if="workspace.tabs.length > 0"
      :value="workspace.activeId"
      :type="tabsProps.type"
      :size="tabsProps.size"
      :closable="tabsProps.closable"
      :animated="false"
      :tabs-padding="8"
      pane-class="workspace-pane"
      @close="onClose"
      @update:value="onTabsUpdate"
    >
      <n-tab-pane
        v-for="tab in workspace.tabs"
        :key="tab.id"
        :name="tab.id"
        :tab="renderTabLabel(tab)"
        display-directive="show"
      >
        <QueryTab v-if="tab.type === 'query'" :tab="tab" />
        <TableDataTab v-else-if="tab.type === 'table-data'" :tab="tab" />
        <TableDesignTab v-else :tab="tab" />
      </n-tab-pane>
    </n-tabs>

    <div v-else class="welcome">
      <div class="welcome-card">
        <IconBolt :size="52" class="welcome-icon" />
        <h1 class="welcome-title">TDengine Viewer</h1>
        <p class="welcome-sub">
          优雅、高性能的 TDengine 图形化客户端
        </p>
        <div class="welcome-actions">
          <n-button type="primary" @click="emit('new-connection')">
            新建连接
          </n-button>
          <n-button tertiary @click="newQuery">打开查询窗口</n-button>
        </div>
        <p class="welcome-tip">
          提示：点击左侧连接名称即可登录服务器 · 双击表名浏览数据 · 右键查看更多操作
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.workspace {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: var(--app-body);
}

.workspace :deep(.n-tabs) {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.workspace :deep(.n-tabs .n-tab-nav) {
  flex-shrink: 0;
  padding-top: 6px;
}

.workspace :deep(.n-tabs .n-tabs-pane-wrapper) {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.workspace :deep(.n-tabs .n-tabs-pane-wrapper .n-tab-pane) {
  height: 100%;
  padding: 0;
  box-sizing: border-box;
}

.tab-label {
  display: inline-flex;
  align-items: center;
  max-width: 180px;
  overflow: hidden;
}

.welcome {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.welcome-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 40px 60px;
}

.welcome-icon {
  color: #34d399;
  margin-bottom: 12px;
  filter: drop-shadow(0 0 24px rgba(52, 211, 153, 0.45));
}

.welcome-title {
  font-size: 22px;
  font-weight: 700;
  letter-spacing: 0.5px;
}

.welcome-sub {
  opacity: 0.55;
  font-size: 13px;
  margin-bottom: 22px;
}

.welcome-actions {
  display: flex;
  gap: 10px;
}

.welcome-tip {
  margin-top: 28px;
  font-size: 12px;
  opacity: 0.4;
}
</style>
