<script setup lang="ts">
import { computed, h, type VNodeChild } from "vue";
import {
  NDataTable,
  NEllipsis,
  NTag,
  type DataTableColumns,
} from "naive-ui";
import type { QueryResult } from "@/types";

interface RowObj {
  __i: number;
  [key: string]: unknown;
}

const props = defineProps<{
  result: QueryResult;
  /** when true, sorting is delegated to the parent via the sort event */
  remoteSort?: boolean;
}>();

const emit = defineEmits<{
  (
    e: "sort",
    payload: { field: string; order: "ascend" | "descend" | false }
  ): void;
}>();

const data = computed<RowObj[]>(() =>
  props.result.rows.map((row, i) => {
    const obj: RowObj = { __i: i };
    row.forEach((v, j) => {
      obj[`c${j}`] = v;
    });
    return obj;
  })
);

function formatNumber(v: number): string {
  if (Number.isInteger(v)) return String(v);
  return String(v);
}

function renderCell(v: unknown): VNodeChild {
  if (v === null || v === undefined) {
    return h("span", { class: "cell-null" }, "NULL");
  }
  if (typeof v === "number") {
    return h("span", { class: "cell-num" }, formatNumber(v));
  }
  if (typeof v === "boolean") {
    return h("span", { class: "cell-bool" }, v ? "true" : "false");
  }
  const s = String(v);
  if (s === "") return h("span", { class: "cell-empty" }, "␀");
  return h(
    NEllipsis,
    { style: "max-width: 100%; user-select: text" },
    { default: () => s }
  );
}

function makeLocalSorter(idx: number) {
  return (a: RowObj, b: RowObj) => {
    const va = a[`c${idx}`];
    const vb = b[`c${idx}`];
    if (va == null) return -1;
    if (vb == null) return 1;
    if (typeof va === "number" && typeof vb === "number") return va - vb;
    return String(va).localeCompare(String(vb));
  };
}

function estimateWidth(idx: number): number {
  const field = props.result.fields[idx];
  let maxLen = field ? field.name.length : 8;
  const sample = Math.min(props.result.rows.length, 40);
  for (let i = 0; i < sample; i++) {
    const v = props.result.rows[i][idx];
    if (v !== null && v !== undefined) {
      const len = String(v).length;
      if (len > maxLen) maxLen = len;
    }
  }
  return Math.min(400, Math.max(96, maxLen * 8 + 42));
}

const columns = computed<DataTableColumns<RowObj>>(() => {
  const cols: DataTableColumns<RowObj> = [
    {
      title: "#",
      key: "__i",
      width: 62,
      align: "right",
      render: (row) =>
        h("span", { class: "row-index" }, String(row.__i + 1)),
      sorter: undefined,
    },
  ];
  props.result.fields.forEach((f, idx) => {
    cols.push({
      title: () =>
        h("span", { class: "col-title" }, [
          f.name,
          h("span", { class: "col-type" }, f.ty),
        ]),
      key: `c${idx}`,
      width: estimateWidth(idx),
      render: (row) => renderCell(row[`c${idx}`]),
      sorter: props.remoteSort ? true : makeLocalSorter(idx),
      ellipsis: { tooltip: false },
    });
  });
  return cols;
});

function onSorterChange(sorter: unknown) {
  if (!props.remoteSort) return;
  // naive sends a sorter object { column, order } or an array
  const s = Array.isArray(sorter) ? sorter[0] : sorter;
  if (s && typeof s === "object" && "order" in s && s.column) {
    const key = String(s.column.key ?? "");
    const idx = key.startsWith("c") ? Number(key.slice(1)) : NaN;
    if (!Number.isNaN(idx)) {
      emit("sort", {
        field: props.result.fields[idx].name,
        order: (s.order as "ascend" | "descend" | false) ?? false,
      });
    }
  }
}

const totalRows = computed(() => props.result.rows.length);
</script>

<template>
  <div class="result-grid">
    <n-data-table
      class="grid-table"
      :columns="columns"
      :data="data"
      :row-key="(row: RowObj) => row.__i"
      :bordered="false"
      size="small"
      flex-height
      virtual-scroll
      :single-line="false"
      @update:sorter="onSorterChange"
    />
    <div class="grid-status">
      <span>{{ totalRows.toLocaleString() }} 行</span>
      <span class="sep">·</span>
      <span>{{ result.elapsedMs.toFixed(1) }} ms</span>
      <template v-if="result.affected">
        <span class="sep">·</span>
        <span>affected {{ result.affected }}</span>
      </template>
      <n-tag
        v-if="result.truncated"
        type="warning"
        size="small"
        :bordered="false"
        class="trunc-tag"
      >
        结果已截断（达到行数上限）
      </n-tag>
      <span class="grid-hint">点击列头排序</span>
    </div>
  </div>
</template>

<style scoped>
.result-grid {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.grid-table {
  flex: 1;
  min-height: 0;
}

.grid-status {
  height: 28px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  border-top: 1px solid var(--app-border);
  font-size: 12px;
  opacity: 0.75;
  background: var(--app-card);
}

.sep {
  opacity: 0.35;
}

.trunc-tag {
  margin-left: 4px;
}

.grid-hint {
  margin-left: auto;
  opacity: 0.35;
  font-size: 11px;
}

:deep(.col-title) {
  display: inline-flex;
  align-items: baseline;
  gap: 6px;
}

:deep(.col-type) {
  font-size: 10px;
  opacity: 0.42;
  font-weight: 400;
  font-family: "SF Mono", Menlo, Consolas, monospace;
}

:deep(.cell-null) {
  color: #7c8590;
  font-style: italic;
  font-size: 11px;
  opacity: 0.7;
}

:deep(.cell-num) {
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-size: 12px;
  text-align: right;
}

:deep(.cell-bool) {
  color: #38bdf8;
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-size: 12px;
}

:deep(.cell-empty) {
  opacity: 0.3;
}

:deep(.row-index) {
  opacity: 0.35;
  font-size: 11px;
  font-family: "SF Mono", Menlo, Consolas, monospace;
}

:deep(.n-data-table-th) {
  white-space: nowrap;
}

:deep(td) {
  white-space: nowrap;
}
</style>
