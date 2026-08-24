<script setup lang="ts">
import { computed, h, onMounted, reactive, ref } from "vue";
import {
  NButton,
  NDataTable,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NModal,
  NSelect,
  NSpace,
  NTag,
  NTooltip,
  useDialog,
  useMessage,
  type DataTableColumns,
} from "naive-ui";
import { describeTable, executeBatch, showCreateTable } from "@/api";
import { quoteIdent } from "@/utils/sql";
import type { ColumnMeta } from "@/types";
import type { WorkspaceTab } from "@/stores/workspace";
import {
  IconCopy,
  IconPencil,
  IconPlus,
  IconRefresh,
  IconTrash,
} from "@/components/icons";

const props = defineProps<{ tab: WorkspaceTab }>();

const message = useMessage();
const dialog = useDialog();

const columns = ref<ColumnMeta[]>([]);
const ddl = ref("");
const loading = ref(false);

const isStable = computed(() => props.tab.kind === "stable");
const alterObj = computed(() => (isStable.value ? "STABLE" : "TABLE"));
const fullTarget = computed(
  () => `${quoteIdent(props.tab.db!)}.${quoteIdent(props.tab.table!)}`
);

async function load() {
  loading.value = true;
  try {
    const [cols, create] = await Promise.all([
      describeTable(props.tab.connId, props.tab.db!, props.tab.table!),
      showCreateTable(props.tab.connId, props.tab.db!, props.tab.table!, props.tab.kind ?? "table").catch(() => ""),
    ]);
    columns.value = cols;
    ddl.value = create;
  } catch (e) {
    message.error(String(e));
  } finally {
    loading.value = false;
  }
}

onMounted(load);

// TDengine type options for ADD COLUMN
const TYPE_OPTIONS = [
  { label: "TIMESTAMP", value: "TIMESTAMP" },
  { label: "BOOL", value: "BOOL" },
  { label: "TINYINT", value: "TINYINT" },
  { label: "SMALLINT", value: "SMALLINT" },
  { label: "INT", value: "INT" },
  { label: "BIGINT", value: "BIGINT" },
  { label: "FLOAT", value: "FLOAT" },
  { label: "DOUBLE", value: "DOUBLE" },
  { label: "VARCHAR(n)", value: "VARCHAR" },
  { label: "NCHAR(n)", value: "NCHAR" },
  { label: "BINARY(n)", value: "BINARY" },
];
const LENGTH_TYPES = new Set(["VARCHAR", "NCHAR", "BINARY"]);

const VAR_TYPES = new Set(["VARCHAR", "NCHAR", "BINARY", "VARBINARY"]);

function isPrimary(idx: number): boolean {
  return idx === 0 && columns.value[0]?.ty.toUpperCase().includes("TIMESTAMP");
}

// ---------- add column ----------

const showAdd = ref(false);
const addForm = reactive({
  name: "",
  type: "VARCHAR",
  length: 64,
  comment: "",
});

function openAdd() {
  Object.assign(addForm, { name: "", type: "VARCHAR", length: 64, comment: "" });
  showAdd.value = true;
}

async function confirmAdd() {
  if (!addForm.name.trim()) {
    message.warning("请填写字段名");
    return;
  }
  const typeStr = LENGTH_TYPES.has(addForm.type)
    ? `${addForm.type}(${addForm.length})`
    : addForm.type;
  const comment = addForm.comment.trim()
    ? ` COMMENT '${addForm.comment.trim().replace(/'/g, "''")}'`
    : "";
  const sql = `ALTER ${alterObj.value} ${fullTarget.value} ADD COLUMN ${quoteIdent(
    addForm.name.trim()
  )} ${typeStr}${comment}`;
  dialog.warning({
    title: "添加字段",
    content: `将执行：\n${sql}`,
    positiveText: "执行",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await executeBatch({ connId: props.tab.connId, sqls: [sql] });
        message.success("字段已添加");
        showAdd.value = false;
        await load();
      } catch (e) {
        message.error(String(e));
      }
    },
  });
}

// ---------- drop column ----------

function dropColumn(col: ColumnMeta) {
  const sql = `ALTER ${alterObj.value} ${fullTarget.value} DROP COLUMN ${quoteIdent(col.name)}`;
  dialog.error({
    title: "删除字段",
    content: `确定删除字段 “${col.name}” 吗？该列所有数据将被删除！\n将执行：${sql}`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await executeBatch({ connId: props.tab.connId, sqls: [sql] });
        message.success("字段已删除");
        await load();
      } catch (e) {
        message.error(String(e));
      }
    },
  });
}

// ---------- modify (widen) column ----------

const showModify = ref(false);
const modifyTarget = ref<ColumnMeta | null>(null);
const modifyForm = reactive({ length: 0 });

function openModify(col: ColumnMeta) {
  modifyTarget.value = col;
  modifyForm.length = Math.max(col.length, col.length * 2, 64);
  showModify.value = true;
}

async function confirmModify() {
  const col = modifyTarget.value;
  if (!col) return;
  if (modifyForm.length <= col.length) {
    message.warning("TDengine 仅支持加宽字段，新长度需大于当前长度");
    return;
  }
  const base = col.ty.toUpperCase().split("(")[0];
  const sql = `ALTER ${alterObj.value} ${fullTarget.value} MODIFY COLUMN ${quoteIdent(
    col.name
  )} ${base}(${modifyForm.length})`;
  try {
    await executeBatch({ connId: props.tab.connId, sqls: [sql] });
    message.success("字段类型已加宽");
    showModify.value = false;
    await load();
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- table columns ----------

const tableColumns = computed<DataTableColumns<ColumnMeta & { __i: number }>>(
  () => [
    { title: "#", key: "__i", width: 52, align: "right" },
    {
      title: "字段名",
      key: "name",
      render: (row) =>
        h("span", { class: ["col-name", isPrimary(row.__i) ? "col-primary" : ""] }, [
          row.name,
          isPrimary(row.__i)
            ? h("span", { class: "pk-badge" }, "主键")
            : null,
        ]),
    },
    {
      title: "类型",
      key: "ty",
      width: 160,
      render: (row) =>
        h("span", { class: "col-type mono" }, row.ty),
    },
    { title: "长度", key: "length", width: 90, align: "right" },
    {
      title: "属性",
      key: "note",
      width: 110,
      render: (row) =>
        row.note.toUpperCase().includes("TAG")
          ? h(NTag, { size: "small", type: "info", bordered: false }, { default: () => "TAG 标签" })
          : h("span", { style: "opacity:0.3" }, "—"),
    },
    {
      title: "操作",
      key: "actions",
      width: 150,
      align: "center",
      render: (row) =>
        h(NSpace, { size: 4, justify: "center", wrap: false }, () => {
          const actions = [];
          if (VAR_TYPES.has(row.ty.toUpperCase().split("(")[0]) && !isPrimary(row.__i)) {
            actions.push(
              h(
                NTooltip,
                {},
                {
                  trigger: () =>
                    h(
                      NButton,
                      {
                        size: "tiny",
                        quaternary: true,
                        type: "warning",
                        onClick: () => openModify(row),
                      },
                      { icon: () => h(IconPencil, { size: 12 }) }
                    ),
                  default: () => "加宽字段（修改类型长度）",
                }
              )
            );
          }
          if (!isPrimary(row.__i)) {
            actions.push(
              h(
                NTooltip,
                {},
                {
                  trigger: () =>
                    h(
                      NButton,
                      {
                        size: "tiny",
                        quaternary: true,
                        type: "error",
                        onClick: () => dropColumn(row),
                      },
                      { icon: () => h(IconTrash, { size: 12 }) }
                    ),
                  default: () => "删除字段",
                }
              )
            );
          }
          return actions;
        }),
    },
  ]
);

const tableData = computed(() =>
  columns.value.map((c, i) => ({ ...c, __i: i }))
);

function rowKey(row: ColumnMeta & { __i: number }) {
  return row.__i;
}

async function copyDdl() {
  if (!ddl.value) return;
  await navigator.clipboard.writeText(ddl.value);
  message.success("建表语句已复制");
}
</script>

<template>
  <div class="design-tab">
    <div class="t-toolbar">
      <n-button size="small" tertiary :loading="loading" @click="load">
        <template #icon>
          <IconRefresh :size="13" />
        </template>
        刷新
      </n-button>
      <n-button size="small" type="primary" @click="openAdd">
        <template #icon>
          <IconPlus :size="13" />
        </template>
        添加字段
      </n-button>
      <span class="t-hint">
        TDengine 支持添加字段、删除字段、加宽变长字段；时间戳主键列不可修改
      </span>
    </div>

    <div class="t-main">
      <div class="t-table">
        <n-data-table
          :columns="tableColumns"
          :data="tableData"
          :row-key="rowKey"
          :bordered="false"
          size="small"
          :loading="loading"
          flex-height
        />
      </div>

      <div class="t-ddl">
        <div class="t-ddl-header">
          <span>建表语句</span>
          <n-button size="tiny" quaternary :disabled="!ddl" @click="copyDdl">
            <template #icon>
              <IconCopy :size="12" />
            </template>
            复制
          </n-button>
        </div>
        <pre class="t-ddl-content mono">{{ ddl || "（无）" }}</pre>
      </div>
    </div>

    <!-- add column modal -->
    <n-modal
      v-model:show="showAdd"
      title="添加字段"
      preset="dialog"
      style="width: 420px"
      positive-text="下一步（生成 SQL）"
      negative-text="取消"
      @positive-click="confirmAdd"
    >
      <n-form label-placement="left" label-width="80" style="margin-top: 8px">
        <n-form-item label="字段名" required>
          <n-input v-model:value="addForm.name" placeholder="如 voltage" />
        </n-form-item>
        <n-form-item label="类型">
          <n-select v-model:value="addForm.type" :options="TYPE_OPTIONS" />
        </n-form-item>
        <n-form-item v-if="LENGTH_TYPES.has(addForm.type)" label="长度">
          <n-input-number
            v-model:value="addForm.length"
            :min="1"
            :max="65517"
            style="width: 100%"
          />
        </n-form-item>
        <n-form-item label="注释">
          <n-input v-model:value="addForm.comment" placeholder="（可选）" />
        </n-form-item>
      </n-form>
    </n-modal>

    <!-- modify column modal -->
    <n-modal
      v-model:show="showModify"
      title="加宽字段"
      preset="dialog"
      style="width: 420px"
      positive-text="执行"
      negative-text="取消"
      @positive-click="confirmModify"
    >
      <div v-if="modifyTarget" style="padding: 8px 0">
        <p style="margin-bottom: 12px">
          字段 <b>{{ modifyTarget.name }}</b> 当前类型
          <span class="mono">{{ modifyTarget.ty }}</span>
        </p>
        <n-form label-placement="left" label-width="80">
          <n-form-item label="新长度">
            <n-input-number
              v-model:value="modifyForm.length"
              :min="(modifyTarget.length || 0) + 1"
              style="width: 100%"
            />
          </n-form-item>
        </n-form>
      </div>
    </n-modal>
  </div>
</template>

<style scoped>
.design-tab {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.t-toolbar {
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  border-bottom: 1px solid var(--app-border);
  background: var(--app-card);
}

.t-hint {
  margin-left: auto;
  font-size: 11px;
  opacity: 0.38;
}

.t-main {
  flex: 1;
  display: flex;
  min-height: 0;
}

.t-table {
  flex: 1;
  min-width: 0;
  padding: 0;
}

.t-table :deep(.n-data-table) {
  height: 100%;
}

.t-ddl {
  width: 340px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--app-border);
  background: var(--app-card);
}

.t-ddl-header {
  height: 32px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 0 12px;
  font-size: 12px;
  opacity: 0.85;
  border-bottom: 1px solid var(--app-border);
}

.t-ddl-content {
  flex: 1;
  margin: 0;
  padding: 10px 14px;
  font-size: 12px;
  line-height: 1.6;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
  user-select: text;
  opacity: 0.85;
}

:deep(.col-name) {
  font-weight: 500;
}

:deep(.col-primary) {
  color: #eab308;
}

:deep(.pk-badge) {
  margin-left: 6px;
  font-size: 10px;
  opacity: 0.7;
  border: 1px solid currentColor;
  border-radius: 3px;
  padding: 0 3px;
}

:deep(.col-type) {
  opacity: 0.75;
  font-size: 12px;
}
</style>
