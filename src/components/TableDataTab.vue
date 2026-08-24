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
import type { FilterCond, QueryResult } from "@/types";
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
// 各列筛选条件：field -> FilterCond | null
const filters = ref<Record<string, FilterCond | null>>({});

const full = () =>
  `${quoteIdent(props.tab.db!)}.${quoteIdent(props.tab.table!)}`;

// 列名 -> SQL 表达式。DECIMAL 列驱动暂不支持，CAST 成 VARCHAR 显示。
// select 用于 SELECT 列表（带 AS 别名）；plain 用于 ORDER BY / WHERE（不能带别名）
const colExpr = ref<Map<string, { select: string; plain: string }>>(new Map());

async function loadColumns() {
  try {
    const cols = await describeTable(
      props.tab.connId,
      props.tab.db!,
      props.tab.table!
    );
    const m = new Map<string, { select: string; plain: string }>();
    for (const c of cols) {
      const q = quoteIdent(c.name);
      if (c.ty.toUpperCase().startsWith("DECIMAL")) {
        m.set(c.name, {
          select: `CAST(${q} AS VARCHAR) AS ${q}`,
          plain: `CAST(${q} AS VARCHAR)`,
        });
      } else {
        m.set(c.name, { select: q, plain: q });
      }
    }
    colExpr.value = m;
  } catch {
    colExpr.value = new Map();
  }
}

function selectList(): string {
  if (colExpr.value.size === 0) return "*";
  return [...colExpr.value.values()].map((e) => e.select).join(", ");
}

function plainExpr(name: string): string {
  return colExpr.value.get(name)?.plain ?? quoteIdent(name);
}

// 转义字符串值中的单引号，避免拼接 SQL 语法错误
function escapeSqlValue(v: string): string {
  return v.replace(/'/g, "''");
}

function buildWhere(): string {
  const conds: string[] = [];
  for (const [name, f] of Object.entries(filters.value)) {
    if (!f || !f.value) continue;
    const col = plainExpr(name);
    const v = escapeSqlValue(f.value);
    switch (f.op) {
      case "contains":
        conds.push(`${col} LIKE '%${v}%'`);
        break;
      case "notcontains":
        conds.push(`${col} NOT LIKE '%${v}%'`);
        break;
      case "eq":
        conds.push(`${col} = '${v}'`);
        break;
      case "neq":
        conds.push(`${col} != '${v}'`);
        break;
      case "gt":
        conds.push(`${col} > '${v}'`);
        break;
      case "ge":
        conds.push(`${col} >= '${v}'`);
        break;
      case "lt":
        conds.push(`${col} < '${v}'`);
        break;
      case "le":
        conds.push(`${col} <= '${v}'`);
        break;
    }
  }
  return conds.length ? ` WHERE ${conds.join(" AND ")}` : "";
}

// 统一数据加载：withCount 时同时执行 COUNT（筛选/刷新后行数会变）
async function loadPage(withCount: boolean) {
  loading.value = true;
  try {
    let sql = `SELECT ${selectList()} FROM ${full()}${buildWhere()}`;
    if (sortField.value && sortOrder.value) {
      sql += ` ORDER BY ${plainExpr(sortField.value)} ${
        sortOrder.value === "ascend" ? "ASC" : "DESC"
      }`;
    }
    sql += ` LIMIT ${pageSize.value} OFFSET ${(page.value - 1) * pageSize.value}`;
    // COUNT 与 SELECT 合并为一次调用：后端按连接串行执行，
    // 避免同一 WS 连接上的并发查询（旧版 taosAdapter 不支持）
    const sqls = withCount
      ? [`SELECT COUNT(*) FROM ${full()}${buildWhere()}`, sql]
      : [sql];
    const res = await executeBatch({
      connId: props.tab.connId,
      sqls,
      maxRows: pageSize.value,
    });
    if (withCount) {
      const v = res[0].rows[0]?.[0];
      total.value = typeof v === "number" ? v : Number(v ?? 0);
      result.value = res[1];
    } else {
      result.value = res[0];
    }
  } catch (e) {
    message.error(String(e));
  } finally {
    loading.value = false;
  }
}

async function refresh() {
  await loadColumns();
  await loadPage(true);
}

function onSort(payload: { field: string; order: "ascend" | "descend" | false }) {
  sortField.value = payload.order ? payload.field : null;
  sortOrder.value = payload.order;
  page.value = 1;
  // 排序不改变行数，无需重新 COUNT
  loadPage(false);
}

function onFilter(payload: { field: string; cond: FilterCond | null }) {
  if (payload.cond) {
    filters.value = { ...filters.value, [payload.field]: payload.cond };
  } else {
    const next = { ...filters.value };
    delete next[payload.field];
    filters.value = next;
  }
  page.value = 1;
  // 筛选改变行数，需要重新 COUNT
  loadPage(true);
}

function clearFilters() {
  filters.value = {};
  page.value = 1;
  loadPage(true);
}

function onPageChange(p: number) {
  page.value = p;
  loadPage(false);
}

function onPageSizeChange(size: number) {
  pageSize.value = size;
  page.value = 1;
  loadPage(true);
}

const activeFilterCount = () =>
  Object.values(filters.value).filter(Boolean).length;

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
      <n-tag
        v-if="activeFilterCount() > 0"
        size="small"
        :bordered="false"
        type="success"
        closable
        @close="clearFilters"
      >
        筛选中 × {{ activeFilterCount() }}
      </n-tag>
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
        :current-sort="{ field: sortField ?? '', order: sortOrder }"
        :filters="filters"
        @sort="onSort"
        @filter="onFilter"
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
