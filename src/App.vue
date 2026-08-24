<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  NConfigProvider,
  NDialogProvider,
  NGlobalStyle,
  NMessageProvider,
  darkTheme,
  type GlobalThemeOverrides,
} from "naive-ui";
import TopBar from "@/components/TopBar.vue";
import Sidebar from "@/components/Sidebar.vue";
import Workspace from "@/components/Workspace.vue";
import ConnectionDialog from "@/components/ConnectionDialog.vue";
import { useUiStore } from "@/stores/ui";
import { useConnectionsStore } from "@/stores/connections";
import type { ConnectionConfig } from "@/types";

const ui = useUiStore();
const connStore = useConnectionsStore();

const theme = computed(() => (ui.theme === "dark" ? darkTheme : null));

const darkOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: "#34d399",
    primaryColorHover: "#6ee7b7",
    primaryColorPressed: "#10b981",
    primaryColorSuppl: "#6ee7b7",
    bodyColor: "#0e1013",
    cardColor: "#15181d",
    modalColor: "#171b21",
    popoverColor: "#1b2027",
    tableColor: "#131519",
    tableHeaderColor: "#171b21",
    inputColor: "#1b2027",
    actionColor: "#1b2027",
    hoverColor: "rgba(52, 211, 153, 0.09)",
    borderColor: "#252a32",
    dividerColor: "#22262d",
    scrollbarColor: "rgba(255, 255, 255, 0.14)",
    scrollbarColorHover: "rgba(255, 255, 255, 0.24)",
    borderRadius: "6px",
    fontSize: "13px",
  },
};

const lightOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: "#0d9488",
    primaryColorHover: "#14b8a6",
    primaryColorPressed: "#0f766e",
    primaryColorSuppl: "#14b8a6",
    borderRadius: "6px",
    fontSize: "13px",
  },
};

const themeOverrides = computed(() =>
  ui.theme === "dark" ? darkOverrides : lightOverrides
);

// connection dialog control
const showConnDialog = ref(false);
const editingConn = ref<ConnectionConfig | null>(null);

function openNewConnection() {
  editingConn.value = null;
  showConnDialog.value = true;
}

function openEditConnection(config: ConnectionConfig) {
  editingConn.value = config;
  showConnDialog.value = true;
}

onMounted(() => {
  connStore.init();
});

// 侧边栏拖拽调宽
function startResize(e: MouseEvent) {
  e.preventDefault();
  const onMove = (ev: MouseEvent) => {
    ui.setSidebarWidth(ev.clientX);
  };
  const onUp = () => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
    document.body.classList.remove("resizing");
  };
  document.body.classList.add("resizing");
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}
</script>

<template>
  <n-config-provider
    :theme="theme"
    :theme-overrides="themeOverrides"
    style="height: 100%"
  >
    <n-global-style />
    <n-message-provider placement="bottom-right">
      <n-dialog-provider>
        <div class="app-shell" :class="{ 'app-light': ui.theme === 'light' }">
          <TopBar
            @new-connection="openNewConnection"
            @new-query="connStore.configs.length > 0 || openNewConnection()"
          />
          <div class="app-body">
            <Sidebar
              :style="{ width: `${ui.sidebarWidth}px` }"
              @new-connection="openNewConnection"
              @edit-connection="openEditConnection"
            />
            <div
              class="sidebar-splitter"
              @mousedown="startResize"
              @dblclick="ui.setSidebarWidth(250)"
            />
            <Workspace />
          </div>
        </div>
        <ConnectionDialog
          v-model:show="showConnDialog"
          :editing="editingConn"
        />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<style scoped>
.app-shell {
  /* app-level design tokens (dark) */
  --app-border: #242931;
  --app-card: #15181d;
  --app-body: #0e1013;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--app-body);
  color: rgba(255, 255, 255, 0.82);
}

.app-shell.app-light {
  --app-border: #e2e5ea;
  --app-card: #ffffff;
  --app-body: #f4f5f7;
  color: #24292f;
}

.app-body {
  flex: 1;
  display: flex;
  min-height: 0;
}

.sidebar-splitter {
  width: 4px;
  flex-shrink: 0;
  margin: 0 -2px;
  cursor: col-resize;
  z-index: 10;
  position: relative;
  background: transparent;
  transition: background 0.15s;
}

.sidebar-splitter:hover,
.sidebar-splitter:active {
  background: var(--app-border);
}

:global(body.resizing) {
  cursor: col-resize;
  user-select: none;
}
</style>
