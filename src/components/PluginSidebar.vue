<template>
  <div class="sidebar-inner">
    <div class="logo">
      <el-icon :size="24" color="#3b82f6"><Switch /></el-icon>
      <span class="logo-text">AppDataHub</span>
    </div>
    <el-menu
      :default-active="pluginStore.activePluginId"
      @select="onSelect"
      class="plugin-menu"
    >
      <el-menu-item
        v-for="p in pluginStore.plugins"
        :key="p.id"
        :index="p.id"
      >
        <el-icon><Box /></el-icon>
        <span>{{ p.name }}</span>
        <el-tag v-if="p.is_builtin" size="small" type="info">内置</el-tag>
      </el-menu-item>
    </el-menu>
    <div class="sidebar-footer">
      <el-button text size="small" @click="goSettings">
        <el-icon><Setting /></el-icon>
        设置
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from "vue-router";
import { Switch, Box, Setting } from "@element-plus/icons-vue";
import { usePluginStore } from "@/stores/plugin";

const router = useRouter();
const pluginStore = usePluginStore();

function onSelect(index: string) {
  pluginStore.selectPlugin(index);
  router.push("/");
}

function goSettings() {
  router.push("/settings");
}
</script>

<style scoped>
.sidebar-inner {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px;
  border-bottom: 1px solid var(--el-border-color-light);
}

.logo-text {
  font-weight: 600;
  font-size: 15px;
}

.plugin-menu {
  flex: 1;
  border-right: none;
}

.sidebar-footer {
  padding: 8px 12px;
  border-top: 1px solid var(--el-border-color-light);
}
</style>
