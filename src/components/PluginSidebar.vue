<template>
  <div class="sidebar-inner">
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
      </el-menu-item>
    </el-menu>
    <div class="sidebar-footer">
      <el-button text size="small" @click="goPlugins">
        <el-icon><Box /></el-icon>
        插件管理
      </el-button>
      <el-button text size="small" @click="goSettings">
        <el-icon><Setting /></el-icon>
        设置
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from "vue-router";
import { Box, Setting } from "@element-plus/icons-vue";
import { usePluginStore } from "@/stores/plugin";

const router = useRouter();
const pluginStore = usePluginStore();

function onSelect(index: string) {
  pluginStore.selectPlugin(index);
  router.push("/");
}

function goPlugins() {
  router.push("/plugins");
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

.plugin-menu {
  flex: 1;
  border-right: none;
}

.sidebar-footer {
  padding: 8px 12px;
  border-top: 1px solid var(--el-border-color-light);
}
</style>
