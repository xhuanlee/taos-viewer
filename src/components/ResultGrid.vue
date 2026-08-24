<script setup lang="ts">
import { computed, defineComponent, h, ref, type PropType, type VNodeChild } from "vue";
import {
  NButton,
  NDataTable,
  NEllipsis,
  NInput,
  NSelect,
  NTag,
  type DataTableColumns,
} from "naive-ui";
import type { FilterCond, QueryResult } from "@/types";
import { IconFilter } from "@/components/icons";

interface RowObj {
  __i: number;
  [key: string]: unknown;
}

const props = defineProps<{
  result: QueryResult;
  /** true 时排序/筛选交给父组件处理（远程模式，表格不本地处理数据） */
  remoteSort?: boolean;
  /** 受控排序状态（remoteSort 时生效） */
  currentSort?: { field: string; order: "ascend" | "descend" | false } | null;
  /** 各列的筛选条件（field -> FilterCond | null） */
  filters?: Record<string, FilterCond | null> | null;
}>();

const emit = defineEmits<{
  (
    e: "sort",
    payload: { field: string; order: "ascend" | "descend" | false }
  ): void;
  (
    e: "filter",
    payload: { field: string; cond: FilterCond | null }
  ): void;
}>();

// ---------- 排序状态 ----------
// 远程模式由父组件受控传入；本地模式（查询结果）内部维护
const localSort = ref<{ field: string; order: "ascend" | "descend" } | null>(null);

const activeSort = computed(() =>
  props.remoteSort ? props.currentSort ?? null : localSort.value
);

function toggleSort(field: string) {
  const cur = activeSort.value?.field === field ? activeSort.value.order : false;
  // 点击循环：无 → 降序 → 升序 → 无
  const next = !cur ? "descend" : cur === "descend" ? "ascend" : false;
  if (props.remoteSort) {
    emit("sort", { field: next ? field : "", order: next });
  } else {
    localSort.value = next ? { field, order: next } : null;
  }
}

const data = computed<RowObj[]>(() => {
  let rows = props.result.rows.map((row, i) => {
    const obj: RowObj = { __i: i };
    row.forEach((v, j) => {
      obj[`c${j}`] = v;
    });
    return obj;
  });
  // 本地模式：在客户端排序（查询结果已全量取回）
  if (!props.remoteSort && localSort.value) {
    const idx = props.result.fields.findIndex(
      (f) => f.name === localSort.value!.field
    );
    if (idx >= 0) {
      const cmp = makeLocalSorter(idx);
      const dir = localSort.value.order === "ascend" ? 1 : -1;
      rows = rows.slice().sort((a, b) => cmp(a, b) * dir);
    }
  }
  return rows;
});

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

// ---------- 列筛选菜单（Navicat 风格：操作符 + 值） ----------

const FILTER_OPS = [
  { label: "包含", value: "contains" },
  { label: "不包含", value: "notcontains" },
  { label: "等于", value: "eq" },
  { label: "不等于", value: "neq" },
  { label: "大于", value: "gt" },
  { label: "大于等于", value: "ge" },
  { label: "小于", value: "lt" },
  { label: "小于等于", value: "le" },
];

const FilterMenu = defineComponent({
  name: "FilterMenu",
  props: {
    cond: { type: Object as PropType<FilterCond | null>, default: null },
  },
  emits: ["apply", "cancel"],
  setup(props, { emit }) {
    const op = ref<FilterCond["op"]>(props.cond?.op ?? "contains");
    const value = ref(props.cond?.value ?? "");

    function apply() {
      const v = value.value.trim();
      if (!v) {
        emit("cancel");
        return;
      }
      emit("apply", { op: op.value, value: v } satisfies FilterCond);
    }

    return () =>
      h(
        "div",
        {
          class: "col-filter-menu",
          // 防止点击菜单内部时 Naive UI 弹层关闭
          onMousedown: (e: MouseEvent) => e.stopPropagation(),
          onClick: (e: MouseEvent) => e.stopPropagation(),
        },
        [
          h(NSelect, {
            value: op.value,
            "onUpdate:value": (v: FilterCond["op"]) => (op.value = v),
            options: FILTER_OPS,
            size: "small",
          }),
          h(NInput, {
            value: value.value,
            "onUpdate:value": (v: string) => (value.value = v),
            size: "small",
            placeholder: "输入筛选值",
            style: "margin-top: 8px",
            autofocus: true,
            onKeydown: (e: KeyboardEvent) => {
              if (e.key === "Enter") apply();
            },
          }),
          h("div", { class: "col-filter-actions" }, [
            h(
              NButton,
              { size: "tiny", quaternary: true, onClick: () => emit("cancel") },
              { default: () => "清除" }
            ),
            h(
              NButton,
              { size: "tiny", type: "primary", onClick: apply },
              { default: () => "应用" }
            ),
          ]),
        ]
      );
  },
});

// ---------- 列定义 ----------

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
    const cond = props.filters?.[f.name] ?? null;
    const isSorted = activeSort.value?.field === f.name;
    cols.push({
      // 点击列名触发排序（Naive UI 虚拟滚动表头的内置排序点击在
      // 2.40.1 中存在 column.sorter 丢失问题，这里自行绑定）
      title: () =>
        h(
          "span",
          {
            class: "col-title col-title--sortable",
            onClick: (e: MouseEvent) => {
              e.stopPropagation();
              toggleSort(f.name);
            },
          },
          [
            f.name,
            h("span", { class: "col-type" }, f.ty),
          ]
        ),
      key: `c${idx}`,
      width: estimateWidth(idx),
      render: (row) => renderCell(row[`c${idx}`]),
      sorter: true,
      // 排序状态统一受控（远程来自父组件，本地来自 localSort），
      // 由 title 的 click 处理排序切换，Naive UI 仅负责图标显示
      sortOrder: isSorted ? activeSort.value?.order ?? false : false,
      // 远程模式启用列筛选（Navicat 风格）
      filter: props.remoteSort ? true : undefined,
      filterOptionValue: cond ? cond.value : null,
      renderFilterIcon: () =>
        h(IconFilter, {
          size: 12,
          style: {
            opacity: cond ? 1 : 0.4,
            color: cond ? "#34d399" : undefined,
          },
        }),
      renderFilterMenu: ({ hide }: { hide: () => void }) =>
        h(FilterMenu, {
          cond,
          onApply: (c: FilterCond) => {
            emit("filter", { field: f.name, cond: c });
            hide();
          },
          onCancel: () => {
            emit("filter", { field: f.name, cond: null });
            hide();
          },
        }),
      ellipsis: { tooltip: false },
    });
  });
  return cols;
});

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
      :remote="remoteSort"
      :single-line="false"
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
      <span class="grid-hint">点击列头排序<template v-if="remoteSort"> · 漏斗筛选</template></span>
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

:deep(.col-title--sortable) {
  cursor: pointer;
  user-select: none;
}

:deep(.col-title--sortable:hover .col-type) {
  opacity: 0.8;
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

<style>
/* 列筛选菜单（renderFilterMenu 渲染在 body 弹层中，需全局样式） */
.col-filter-menu {
  width: 200px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 0;
}

.col-filter-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  margin-top: 10px;
}
</style>
