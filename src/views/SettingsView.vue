<template>
  <div class="settings-view">
    <h2>设置</h2>

    <el-card class="settings-card" shadow="never">
      <template #header>通用</template>
      <el-form label-width="180px">
        <el-form-item label="切换时自动结束应用">
          <el-switch v-model="settings.auto_kill" />
        </el-form-item>
        <el-form-item label="切换后自动启动">
          <el-switch v-model="settings.auto_launch_after_switch" />
        </el-form-item>
        <el-form-item label="主题">
          <el-radio-group v-model="settings.theme">
            <el-radio value="light">浅色</el-radio>
            <el-radio value="dark">深色</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <el-button type="primary" @click="saveSettings">保存设置</el-button>
    </el-card>

    <el-card class="settings-card" shadow="never">
      <template #header>数据</template>
      <el-button @click="onExport">导出数据</el-button>
      <el-button @click="onImport">导入数据</el-button>
      <el-button @click="onOpenDir">打开数据目录</el-button>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { ElMessage } from "element-plus";
import * as api from "@/api";
import { useSettingsStore } from "@/stores/settings";

const settingsStore = useSettingsStore();
const settings = ref({ ...settingsStore.settings });

async function saveSettings() {
  await settingsStore.save(settings.value);
  ElMessage.success("设置已保存");
}

async function onExport() {
  const json = await api.exportData();
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "appdatahub-export.json";
  a.click();
  URL.revokeObjectURL(url);
}

async function onImport() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json";
  input.onchange = async () => {
    const file = input.files?.[0];
    if (!file) return;
    const text = await file.text();
    await api.importData(text);
    ElMessage.success("导入成功");
  };
  input.click();
}

async function onOpenDir() {
  await api.openDataDir();
}
</script>

<style scoped>
.settings-view {
  max-width: 700px;
  margin: 0 auto;
}

.settings-card {
  margin-bottom: 16px;
}
</style>
