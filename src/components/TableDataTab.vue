<script setup lang="ts">
import { onMounted, ref } from "vue";
import {
  NButton,
  NPagination,
  NSelect,
  NSpin,
  NTag,
  useMessage,
} from "naive-ui";
import { describeTable, executeBatch } from "@/api";
import { quoteIdent } from "@/utils/sql";
import type { QueryResult } from "@/types";
import type { WorkspaceTab } from "@/stores/workspace";
import ResultGrid from "@/components/ResultGrid.vue";
import { IconRefresh } from "@/components/icons";

const props = defineProps<{ tab: WorkspaceTab }>();

const message = useMessage();

const page = ref(1);
const pageSize = ref(100);
const total = ref(0);
const result = ref<QueryResult | null>(null);
const loading = ref(false);
const sortField = ref<string | null>(null);
const sortOrder = ref<"ascend" | "descend" | false>(false);

const full = () =>
  `${quoteIdent(props.tab.db!)}.${quoteIdent(props.tab.table!)}`;

// 列名 -> SELECT 表达式。DECIMAL 列驱动暂不支持，CAST 成 VARCHAR 显示
const colExpr = ref<Map<string, string>>(new Map());

async function loadColumns() {
  try {
    const cols = await describeTable(
      props.tab.connId,
      props.tab.db!,
      props.tab.table!
    );
    const m = new Map<string, string>();
    for (const c of cols) {
      m.set(
        c.name,
        c.ty.toUpperCase().startsWith("DECIMAL")
          ? `CAST(${quoteIdent(c.name)} AS VARCHAR) AS ${quoteIdent(c.name)}`
          : quoteIdent(c.name)
      );
    }
    colExpr.value = m;
  } catch {
    colExpr.value = new Map();
  }
}

function selectList(): string {
  if (colExpr.value.size === 0) return "*";
  return [...colExpr.value.values()].join(", ");
}

function orderExpr(name: string): string {
  return colExpr.value.get(name) ?? quoteIdent(name);
}

async function loadData() {
  loading.value = true;
  try {
    let sql = `SELECT ${selectList()} FROM ${full()}`;
    if (sortField.value && sortOrder.value) {
      sql += ` ORDER BY ${orderExpr(sortField.value)} ${
        sortOrder.value === "ascend" ? "ASC" : "DESC"
      }`;
    }
    sql += ` LIMIT ${pageSize.value} OFFSET ${(page.value - 1) * pageSize.value}`;
    const res = await executeBatch({
      connId: props.tab.connId,
      sqls: [sql],
      maxRows: pageSize.value,
    });
    result.value = res[0];
  } finally {
    loading.value = false;
  }
}

async function refresh() {
  // COUNT 与 SELECT 合并为一次调用：后端按连接串行执行，
  // 避免同一 WS 连接上的并发查询（旧版 taosAdapter 不支持）
  loading.value = true;
  try {
    await loadColumns();
    let sql = `SELECT ${selectList()} FROM ${full()}`;
    if (sortField.value && sortOrder.value) {
      sql += ` ORDER BY ${orderExpr(sortField.value)} ${
        sortOrder.value === "ascend" ? "ASC" : "DESC"
      }`;
    }
    sql += ` LIMIT ${pageSize.value} OFFSET ${(page.value - 1) * pageSize.value}`;
    const res = await executeBatch({
      connId: props.tab.connId,
      sqls: [`SELECT COUNT(*) FROM ${full()}`, sql],
      maxRows: pageSize.value,
    });
    const v = res[0].rows[0]?.[0];
    total.value = typeof v === "number" ? v : Number(v ?? 0);
    result.value = res[1];
  } catch (e) {
    message.error(String(e));
  } finally {
    loading.value = false;
  }
}

function onSort(payload: { field: string; order: "ascend" | "descend" | false }) {
  sortField.value = payload.order ? payload.field : null;
  sortOrder.value = payload.order;
  page.value = 1;
  loadData().catch((e) => message.error(String(e)));
}

function onPageChange(p: number) {
  page.value = p;
  loadData().catch((e) => message.error(String(e)));
}

function onPageSizeChange(size: number) {
  pageSize.value = size;
  page.value = 1;
  loadData().catch((e) => message.error(String(e)));
}

onMounted(refresh);
</script>

<template>
  <div class="data-tab">
    <div class="d-toolbar">
      <n-button size="small" tertiary :loading="loading" @click="refresh">
        <template #icon>
          <IconRefresh :size="13" />
        </template>
        刷新
      </n-button>
      <n-tag size="small" :bordered="false" type="info">
        {{ tab.kind === "stable" ? "超级表" : tab.kind === "view" ? "视图" : "普通表" }}
      </n-tag>
      <span class="d-meta mono">{{ tab.db }}.{{ tab.table }}</span>
      <span class="d-total">共 {{ total.toLocaleString() }} 行</span>
      <div class="d-right">
        <span class="d-pagesize-label">每页</span>
        <n-select
          v-model:value="pageSize"
          size="small"
          :options="[50, 100, 200, 500].map((n) => ({ label: String(n), value: n }))"
          style="width: 84px"
          @update:value="onPageSizeChange"
        />
      </div>
    </div>

    <div class="d-grid">
      <n-spin v-if="!result && loading" class="d-spin" size="medium" />
      <ResultGrid
        v-else-if="result"
        :result="result"
        remote-sort
        @sort="onSort"
      />
    </div>

    <div class="d-pagination">
      <n-pagination
        :page="page"
        :page-size="pageSize"
        :item-count="total"
        :page-sizes="[50, 100, 200, 500]"
        show-size-picker
        size="small"
        @update:page="onPageChange"
        @update:page-size="onPageSizeChange"
      />
    </div>
  </div>
</template>

<style scoped>
.data-tab {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.d-toolbar {
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 12px;
  border-bottom: 1px solid var(--app-border);
  background: var(--app-card);
}

.d-meta {
  font-size: 12px;
  opacity: 0.6;
}

.d-total {
  font-size: 12px;
  opacity: 0.55;
}

.d-right {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.d-pagesize-label {
  font-size: 12px;
  opacity: 0.55;
}

.d-grid {
  flex: 1;
  min-height: 0;
  position: relative;
}

.d-spin {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}

.d-pagination {
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 12px;
  border-top: 1px solid var(--app-border);
  background: var(--app-card);
}
</style>
