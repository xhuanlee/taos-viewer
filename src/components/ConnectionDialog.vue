<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import {
  NButton,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NModal,
  NSpace,
  NTag,
  useMessage,
} from "naive-ui";
import { testConnection } from "@/api";
import { useConnectionsStore } from "@/stores/connections";
import type { ConnectionConfig } from "@/types";

const props = defineProps<{
  show: boolean;
  editing: ConnectionConfig | null;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
}>();

const connStore = useConnectionsStore();
const message = useMessage();

const formModel = reactive({
  name: "",
  host: "localhost",
  port: 6041,
  user: "root",
  password: "taosdata",
  database: "",
});

watch(
  () => props.show,
  (show) => {
    if (show) {
      if (props.editing) {
        Object.assign(formModel, {
          name: props.editing.name,
          host: props.editing.host,
          port: props.editing.port,
          user: props.editing.user,
          password: props.editing.password,
          database: props.editing.database ?? "",
        });
      } else {
        Object.assign(formModel, {
          name: "",
          host: "localhost",
          port: 6041,
          user: "root",
          password: "taosdata",
          database: "",
        });
      }
      testState.value = null;
    }
  }
);

const isEdit = computed(() => !!props.editing);

const testState = ref<{ ok: boolean; text: string } | null>(null);
const testing = ref(false);

function buildConfig(): ConnectionConfig {
  return {
    id: props.editing?.id ?? crypto.randomUUID(),
    name: formModel.name.trim() || `${formModel.host}:${formModel.port}`,
    host: formModel.host.trim(),
    port: formModel.port,
    user: formModel.user.trim() || "root",
    password: formModel.password,
    database: formModel.database.trim() || null,
  };
}

async function onTest() {
  testing.value = true;
  testState.value = null;
  try {
    const info = await testConnection(buildConfig());
    testState.value = {
      ok: true,
      text: `连接成功 · TDengine v${info.version} · 服务器时间 ${info.serverTime}`,
    };
  } catch (e) {
    testState.value = { ok: false, text: String(e) };
  } finally {
    testing.value = false;
  }
}

const saving = ref(false);

async function onSave() {
  if (!formModel.host.trim()) {
    message.warning("请填写主机地址");
    return;
  }
  saving.value = true;
  try {
    const config = buildConfig();
    if (isEdit.value) {
      await connStore.updateConfig(config);
      message.success("连接已更新");
    } else {
      await connStore.addConfig(config);
      message.success("连接已保存，点击连接名称即可登录");
    }
    emit("update:show", false);
  } catch (e) {
    message.error(String(e));
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <n-modal
    :show="show"
    :title="isEdit ? '编辑连接' : '新建连接'"
    preset="dialog"
    style="width: 460px"
    :mask-closable="false"
    @update:show="(v: boolean) => emit('update:show', v)"
  >
    <n-form label-placement="left" label-width="86" size="medium" style="margin-top: 8px">
      <n-form-item label="连接名称">
        <n-input
          v-model:value="formModel.name"
          placeholder="我的 TDengine"
        />
      </n-form-item>
      <n-form-item label="主机">
        <n-input v-model:value="formModel.host" placeholder="localhost" />
      </n-form-item>
      <n-form-item label="端口 (WS)">
        <n-input-number
          v-model:value="formModel.port"
          :min="1"
          :max="65535"
          style="width: 100%"
        />
      </n-form-item>
      <n-form-item label="用户名">
        <n-input v-model:value="formModel.user" placeholder="root" />
      </n-form-item>
      <n-form-item label="密码">
        <n-input
          v-model:value="formModel.password"
          type="password"
          show-password-on="click"
          placeholder="taosdata"
        />
      </n-form-item>
      <n-form-item label="默认数据库">
        <n-input
          v-model:value="formModel.database"
          placeholder="（可选）"
        />
      </n-form-item>
    </n-form>

    <div v-if="testState" class="test-result">
      <n-tag :type="testState.ok ? 'success' : 'error'" size="small" :bordered="false">
        {{ testState.ok ? "连接成功" : "连接失败" }}
      </n-tag>
      <span :class="testState.ok ? 'ok-text' : 'err-text'" class="test-text">
        {{ testState.text }}
      </span>
    </div>

    <template #action>
      <n-space :size="8" style="width: 100%" justify="space-between">
        <n-button :loading="testing" @click="onTest">测试连接</n-button>
        <n-space :size="8">
          <n-button quaternary @click="emit('update:show', false)">取消</n-button>
          <n-button type="primary" :loading="saving" @click="onSave">
            {{ isEdit ? "保存" : "创建" }}
          </n-button>
        </n-space>
      </n-space>
    </template>
  </n-modal>
</template>

<style scoped>
.test-result {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  background: rgba(127, 127, 127, 0.08);
  margin-bottom: 4px;
}

.test-text {
  font-size: 12px;
  line-height: 1.5;
  word-break: break-all;
}

.ok-text {
  color: #34d399;
}

.err-text {
  color: #f87171;
}
</style>
