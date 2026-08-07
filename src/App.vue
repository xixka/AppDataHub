<template>
  <el-config-provider :locale="zhCn">
    <el-container class="app-container">
      <!-- 左侧栏 -->
      <el-aside width="200px" class="sidebar">
        <PluginSidebar />
      </el-aside>

      <!-- 主内容 -->
      <el-main class="main-content">
        <router-view />
      </el-main>
    </el-container>
  </el-config-provider>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import { ElConfigProvider } from "element-plus";
import zhCn from "element-plus/es/locale/lang/zh-cn";
import PluginSidebar from "@/components/PluginSidebar.vue";
import { usePluginStore } from "@/stores/plugin";
import { useSettingsStore } from "@/stores/settings";

const pluginStore = usePluginStore();
const settingsStore = useSettingsStore();

onMounted(async () => {
  await Promise.all([pluginStore.load(), settingsStore.load()]);
});
</script>

<style>
body,
html {
  margin: 0;
  padding: 0;
  height: 100vh;
  overflow: hidden;
}

#app {
  height: 100vh;
}

.app-container {
  height: 100vh;
}

.sidebar {
  border-right: 1px solid var(--el-border-color-light);
  background: var(--el-bg-color-page);
  overflow-y: auto;
}

.main-content {
  padding: 20px;
  overflow-y: auto;
}
</style>
