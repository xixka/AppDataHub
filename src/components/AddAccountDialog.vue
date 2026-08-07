<template>
  <el-dialog
    v-model="dialogVisible"
    :title="editing ? '编辑账号' : '添加账号'"
    width="480px"
    @close="onClose"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="80px">
      <el-form-item label="名称" prop="name">
        <el-input v-model="form.name" placeholder="如：工作账号" />
      </el-form-item>
      <el-form-item label="邮箱">
        <el-input v-model="form.email" placeholder="可选" />
      </el-form-item>
      <el-form-item label="备注">
        <el-input v-model="form.note" type="textarea" :rows="2" placeholder="可选" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="dialogVisible = false">取消</el-button>
      <el-button type="primary" :loading="saving" @click="onSave">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from "vue";
import { ElMessage } from "element-plus";
import type { FormInstance, FormRules } from "element-plus";
import { useAccountStore } from "@/stores/account";
import type { AccountMetadata } from "@/types";

const props = defineProps<{
  visible: boolean;
  pluginId: string;
  editing: AccountMetadata | null;
}>();

const emit = defineEmits<{ "update:visible": [v: boolean] }>();

const accountStore = useAccountStore();
const formRef = ref<FormInstance>();
const saving = ref(false);

const dialogVisible = computed({
  get: () => props.visible,
  set: (v) => emit("update:visible", v),
});

const form = reactive({
  name: "",
  email: "",
  note: "",
});

const rules: FormRules = {
  name: [{ required: true, message: "请输入账号名称", trigger: "blur" }],
};

async function onSave() {
  if (!formRef.value) return;
  await formRef.value.validate(async (valid) => {
    if (!valid) return;
    saving.value = true;
    try {
      if (props.editing) {
        await accountStore.update({
          id: props.editing.id,
          name: form.name,
          email: form.email || null,
          note: form.note || null,
          pluginId: props.pluginId,
        });
      } else {
        await accountStore.add({
          name: form.name,
          email: form.email || null,
          note: form.note || null,
          pluginId: props.pluginId,
          machineId: null,
        });
      }
      ElMessage.success(props.editing ? "已更新" : "已添加");
      dialogVisible.value = false;
    } catch (e) {
      ElMessage.error("操作失败: " + e);
    } finally {
      saving.value = false;
    }
  });
}

function onClose() {
  form.name = "";
  form.email = "";
  form.note = "";
}

watch(
  () => props.visible,
  (v) => {
    if (v && props.editing) {
      form.name = props.editing.name;
      form.email = props.editing.email ?? "";
      form.note = props.editing.note ?? "";
    } else if (v) {
      form.name = "";
      form.email = "";
      form.note = "";
    }
  },
);
</script>
