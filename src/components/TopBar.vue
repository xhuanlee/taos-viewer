<script setup lang="ts">
import { computed } from "vue";
import { NButton, NTooltip } from "naive-ui";
import { useMessage } from "naive-ui";
import {
  IconBolt,
  IconMoon,
  IconPlus,
  IconSun,
  IconTerminal,
} from "@/components/icons";
import { useUiStore } from "@/stores/ui";
import { useConnectionsStore } from "@/stores/connections";
import { useWorkspaceStore } from "@/stores/workspace";

const emit = defineEmits<{
  (e: "new-connection"): void;
  (e: "new-query"): void;
}>();

const ui = useUiStore();
const connStore = useConnectionsStore();
const workspace = useWorkspaceStore();
const message = useMessage();

const connectedIds = computed(() =>
  connStore.configs.filter((c) => connStore.serverInfos[c.id])
);

function newQuery() {
  if (connectedIds.value.length === 0) {
    message.warning("请先建立连接");
    emit("new-connection");
    return;
  }
  // prefer the connection of the active tab, else the first connected one
  const activeTab = workspace.tabs.find((t) => t.id === workspace.activeId);
  const connId =
    activeTab?.connId ?? connectedIds.value[0].id;
  const config = connStore.getConfig(connId) ?? connectedIds.value[0];
  workspace.openQueryTab(config.id, config.database ?? undefined);
}
</script>

<template>
  <div class="topbar">
    <div class="brand">
      <IconBolt :size="18" class="brand-icon" />
      <span class="brand-name">TDengine Viewer</span>
    </div>
    <div class="actions">
      <n-button size="small" tertiary @click="emit('new-connection')">
        <template #icon>
          <IconPlus :size="14" />
        </template>
        新建连接
      </n-button>
      <n-button size="small" type="primary" @click="newQuery">
        <template #icon>
          <IconTerminal :size="14" />
        </template>
        新建查询
      </n-button>
      <n-tooltip placement="bottom">
        <template #trigger>
          <n-button size="small" quaternary circle @click="ui.toggleTheme()">
            <template #icon>
              <IconSun v-if="ui.theme === 'dark'" :size="15" />
              <IconMoon v-else :size="15" />
            </template>
          </n-button>
        </template>
        {{ ui.theme === "dark" ? "切换到浅色" : "切换到深色" }}
      </n-tooltip>
    </div>
  </div>
</template>

<style scoped>
.topbar {
  height: 46px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px 0 14px;
  border-bottom: 1px solid var(--app-border);
  background: var(--app-card);
  -webkit-app-region: no-drag;
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
}

.brand-icon {
  color: #34d399;
}

.brand-name {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.2px;
}

.actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
