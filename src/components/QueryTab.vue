<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import {
  EditorState,
  Compartment,
} from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { basicSetup } from "codemirror";
import { sql, PostgreSQL } from "@codemirror/lang-sql";
import { oneDark } from "@codemirror/theme-one-dark";
import {
  NAlert,
  NButton,
  NSelect,
  NSpin,
  NTabPane,
  NTabs,
  NTooltip,
  useMessage,
} from "naive-ui";
import { executeBatch } from "@/api";
import { splitSqlStatements, firstLine } from "@/utils/sql";
import { useUiStore } from "@/stores/ui";
import { useConnectionsStore } from "@/stores/connections";
import type { QueryResult } from "@/types";
import type { WorkspaceTab } from "@/stores/workspace";
import ResultGrid from "@/components/ResultGrid.vue";
import { IconPlay, IconRefresh, IconX } from "@/components/icons";

const props = defineProps<{ tab: WorkspaceTab }>();

const ui = useUiStore();
const connStore = useConnectionsStore();
const message = useMessage();

// ---------- state ----------

const editorHost = ref<HTMLElement | null>(null);
const view = shallowRef<EditorView | null>(null);
const sqlText = ref(
  props.tab.db
    ? `-- 在 ${props.tab.db} 库中执行查询，按 Ctrl/⌘ + Enter 运行\n-- 选中部分文本可仅执行选中语句\nSHOW DATABASES;\n`
    : `-- 按 Ctrl/⌘ + Enter 运行，选中部分文本可仅执行选中语句\nSHOW DATABASES;\n`
);

const db = ref<string | null>(props.tab.db ?? null);
const running = ref(false);
const results = ref<QueryResult[]>([]);
const errorText = ref<string | null>(null);
const activeResult = ref("0");
const maxRows = ref(10000);
const editorFlex = ref(50); // percentage height of editor

// ---------- database options ----------

const dbOptions = computed(() => {
  const dbs = connStore.databases[props.tab.connId] ?? [];
  return [
    { label: "（不指定）", value: "" },
    ...dbs.map((d) => ({ label: d.name, value: d.name })),
  ];
});

watch(db, () => {
  connStore.loadTables(props.tab.connId, db.value ?? "").catch(() => {});
  refreshSchema();
});

const schemaConfig = computed(() => {
  const tbs = connStore.tables[`${props.tab.connId}:${db.value ?? ""}`] ?? [];
  const tables: Record<string, string[]> = {};
  for (const tb of tbs) tables[tb.name] = [];
  return { schema: { [db.value ?? "default"]: { tables } } };
});

const schemaComp = new Compartment();
const themeComp = new Compartment();

function buildExtensions() {
  return [
    keymap.of([
      {
        key: "Mod-Enter",
        preventDefault: true,
        run: () => {
          runSql();
          return true;
        },
      },
    ]),
    EditorView.updateListener.of((u) => {
      if (u.docChanged) {
        sqlText.value = u.state.doc.toString();
      }
    }),
  ];
}

onMounted(() => {
  if (!editorHost.value) return;
  view.value = new EditorView({
    state: EditorState.create({
      doc: sqlText.value,
      extensions: [
        basicSetup,
        sql({ dialect: PostgreSQL, ...schemaConfig.value }),
        themeComp.of(ui.theme === "dark" ? oneDark : []),
        schemaComp.of(sql({ dialect: PostgreSQL, ...schemaConfig.value })),
        ...buildExtensions(),
      ],
    }),
    parent: editorHost.value,
  });
  // preload tables for autocomplete if db selected
  if (db.value) {
    connStore.loadTables(props.tab.connId, db.value).catch(() => {});
  }
});

onBeforeUnmount(() => {
  view.value?.destroy();
});

function refreshSchema() {
  view.value?.dispatch({
    effects: schemaComp.reconfigure(
      sql({ dialect: PostgreSQL, ...schemaConfig.value })
    ),
  });
}

watch(schemaConfig, refreshSchema);

watch(
  () => ui.theme,
  (t) => {
    view.value?.dispatch({
      effects: themeComp.reconfigure(t === "dark" ? oneDark : []),
    });
  }
);

// ---------- execution ----------

async function runSql() {
  if (running.value) return;
  let text = sqlText.value;
  // if there is a selection, execute only the selection
  const sel = view.value?.state.selection.main;
  if (sel && !sel.empty) {
    text = view.value!.state.sliceDoc(sel.from, sel.to);
  }
  const sqls = splitSqlStatements(text);
  if (sqls.length === 0) {
    message.warning("没有可执行的 SQL 语句");
    return;
  }
  running.value = true;
  errorText.value = null;
  results.value = [];
  try {
    const res = await executeBatch({
      connId: props.tab.connId,
      db: db.value || null,
      sqls,
      maxRows: maxRows.value,
    });
    results.value = res;
    activeResult.value = "0";
    const elapsed = res.reduce((s, r) => s + r.elapsedMs, 0);
    const totalRows = res.reduce((s, r) => s + r.rows.length, 0);
    message.success(
      `执行成功：${sqls.length} 条语句 · ${totalRows} 行 · ${elapsed.toFixed(1)} ms`
    );
  } catch (e) {
    errorText.value = String(e);
  } finally {
    running.value = false;
  }
}

// ---------- splitter drag ----------

const dragging = ref(false);

function onDragStart(e: MouseEvent) {
  e.preventDefault();
  // 注意：必须在 mousedown 时保存容器引用，
  // mousemove 回调里访问 e.currentTarget 已失效（为 null）
  const container = (e.currentTarget as HTMLElement)
    .closest(".query-tab") as HTMLElement | null;
  if (!container) return;
  const rect = container.getBoundingClientRect();
  const toolbarH = 40; // 工具栏高度，换算编辑区百分比时扣除
  dragging.value = true;
  const onMove = (ev: MouseEvent) => {
    const usable = rect.height - toolbarH;
    if (usable <= 0) return;
    const pct = ((ev.clientY - rect.top - toolbarH) / usable) * 100;
    editorFlex.value = Math.min(85, Math.max(15, pct));
  };
  const onUp = () => {
    dragging.value = false;
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
    document.body.classList.remove("resizing-rows");
  };
  document.body.classList.add("resizing-rows");
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}

const resultTabs = computed(() =>
  results.value.map((r, i) => ({
    index: i,
    label:
      results.value.length > 1
        ? `结果 ${i + 1}`
        : firstLine(r.sql, 24) || "结果",
  }))
);
</script>

<template>
  <div class="query-tab">
    <!-- toolbar -->
    <div class="q-toolbar">
      <n-button
        type="primary"
        size="small"
        :loading="running"
        @click="runSql"
      >
        <template #icon>
          <IconPlay :size="13" />
        </template>
        运行
        <span class="kbd-hint">⌘↵</span>
      </n-button>

      <n-select
        v-model:value="db"
        size="small"
        :options="dbOptions"
        placeholder="选择数据库"
        style="width: 180px"
        consistent-menu-width
        filterable
      />

      <div class="q-toolbar-right">
        <n-tooltip placement="bottom">
          <template #trigger>
            <span class="row-limit">行数上限 {{ maxRows.toLocaleString() }}</span>
          </template>
          查询结果最多返回的行数（防止超大结果集卡顿）
        </n-tooltip>
        <n-select
          v-model:value="maxRows"
          size="small"
          :options="[
            { label: '1,000', value: 1000 },
            { label: '10,000', value: 10000 },
            { label: '100,000', value: 100000 },
          ]"
          style="width: 110px"
        />
      </div>
    </div>

    <!-- editor -->
    <div class="q-editor" :style="{ flexBasis: editorFlex + '%' }">
      <div ref="editorHost" class="editor-host"></div>
      <n-spin v-if="running" class="editor-loading" size="small" />
    </div>

    <!-- splitter -->
    <div class="q-splitter" @mousedown="onDragStart">
      <div v-if="dragging" class="q-drag-mask"></div>
    </div>

    <!-- results -->
    <div class="q-results">
      <template v-if="errorText">
        <n-alert type="error" :bordered="false" class="q-error">
          <pre class="err-detail">{{ errorText }}</pre>
        </n-alert>
      </template>

      <template v-else-if="results.length > 0">
        <n-tabs
          v-model:value="activeResult"
          type="segment"
          size="small"
          class="q-result-tabs"
        >
          <n-tab-pane
            v-for="rt in resultTabs"
            :key="rt.index"
            :name="String(rt.index)"
            :tab="rt.label"
            display-directive="show"
          >
            <ResultGrid :result="results[rt.index]" />
          </n-tab-pane>
        </n-tabs>
      </template>

      <div v-else class="q-empty">
        <span>执行 SQL 后，结果将在此显示</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.query-tab {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.q-toolbar {
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 12px;
  border-bottom: 1px solid var(--app-border);
  background: var(--app-card);
}

.kbd-hint {
  font-size: 11px;
  opacity: 0.7;
  margin-left: 4px;
}

.q-toolbar-right {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.row-limit {
  font-size: 12px;
  opacity: 0.5;
}

.q-editor {
  position: relative;
  flex-grow: 0;
  flex-shrink: 0;
  min-height: 80px;
  overflow: hidden;
  border-bottom: 1px solid var(--app-border);
}

.editor-host {
  height: 100%;
  overflow: hidden;
}

.editor-host :deep(.cm-editor) {
  height: 100%;
}

.editor-loading {
  position: absolute;
  top: 8px;
  right: 14px;
}

.q-splitter {
  height: 7px;
  flex-shrink: 0;
  cursor: row-resize;
  background: transparent;
  position: relative;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: center;
  border-bottom: 1px solid var(--app-border);
}

.q-splitter::after {
  content: "";
  width: 36px;
  height: 3px;
  border-radius: 2px;
  background: var(--app-border);
  transition: background 0.15s;
}

.q-splitter:hover::after,
.q-splitter:active::after {
  background: #34d399;
}

:global(body.resizing-rows) {
  cursor: row-resize;
  user-select: none;
}

.q-drag-mask {
  position: fixed;
  inset: 0;
  z-index: 10;
}

.q-results {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--app-card);
}

.q-result-tabs {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.q-result-tabs :deep(.n-tabs-tab) {
  font-size: 12px;
}

/* 此版本 Naive UI 无 pane-wrapper，n-tab-pane 是 .n-tabs 直接子元素 */
.q-result-tabs:deep(.n-tabs > .n-tab-pane) {
  flex: 1;
  min-height: 0;
  padding: 0;
}

/* 兼容带 pane-wrapper 的结构 */
.q-result-tabs :deep(.n-tabs-pane-wrapper),
.q-result-tabs :deep(.n-tabs-pane-wrapper .n-tab-pane) {
  height: 100%;
  padding: 0;
}

.q-result-tabs :deep(.n-tabs .n-tabs-nav) {
  padding: 6px 10px 0;
}

.q-result-tabs :deep(.n-tabs .n-tabs-pane-wrapper) {
  padding: 0;
}

.q-error {
  margin: 10px;
}

.err-detail {
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
  user-select: text;
}

.q-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.35;
  font-size: 12px;
}
</style>
