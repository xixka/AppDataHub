<template>
  <div class="plugin-manage">
    <div class="header-bar">
      <h2>插件管理</h2>
    </div>

    <div v-loading="pluginStore.loading">
      <el-empty v-if="pluginStore.plugins.length === 0" description="暂无插件" />
      <div v-else class="plugin-list">
        <el-card
          v-for="p in pluginStore.plugins"
          :key="p.id"
          class="plugin-card"
          :class="{ disabled: !p.enabled }"
          shadow="hover"
        >
          <div class="card-body">
            <div class="card-left">
              <span class="plugin-icon">{{ p.icon || "📦" }}</span>
            </div>
            <div class="card-center">
              <div class="card-name">
                {{ p.name }}
                <el-tag v-if="!p.enabled" size="small" type="danger">已禁用</el-tag>
              </div>
              <div class="card-meta">
                <span>v{{ p.version }}</span>
                <span v-if="p.exe_path">路径: {{ p.exe_path }}</span>
                <span v-else class="warn">未配置路径</span>
              </div>
            </div>
            <div class="card-right">
              <el-switch
                :model-value="p.enabled"
                @update:model-value="(val: boolean) => onToggle(p.id, val)"
                active-text="启用"
                inactive-text="禁用"
                inline-prompt
              />
            </div>
          </div>
        </el-card>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import { ElMessage } from "element-plus";
import { usePluginStore } from "@/stores/plugin";

const pluginStore = usePluginStore();

async function onToggle(pluginId: string, enabled: boolean) {
  try {
    await pluginStore.toggle(pluginId, enabled);
    ElMessage.success(enabled ? "已启用" : "已禁用");
  } catch (e) {
    ElMessage.error("操作失败: " + e);
  }
}

onMounted(() => pluginStore.load());
</script>

<style scoped>
.plugin-manage {
  max-width: 900px;
  margin: 0 auto;
}

.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.header-bar h2 {
  margin: 0;
}

.plugin-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.plugin-card.disabled {
  opacity: 0.6;
}

.card-body {
  display: flex;
  align-items: center;
  gap: 16px;
}

.plugin-icon {
  font-size: 28px;
}

.card-center {
  flex: 1;
}

.card-name {
  font-weight: 600;
  font-size: 15px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.card-meta {
  color: var(--el-text-color-secondary);
  font-size: 13px;
  display: flex;
  gap: 16px;
  margin-top: 4px;
}

.card-meta .warn {
  color: var(--el-color-warning);
}

.card-right {
  display: flex;
  align-items: center;
  gap: 12px;
}
</style>
