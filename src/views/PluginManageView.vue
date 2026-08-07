<template>
  <div class="plugin-manage">
    <div class="header-bar">
      <h2>插件管理</h2>
      <el-button type="primary" @click="showAdd = true">
        <el-icon><Plus /></el-icon> 添加插件
      </el-button>
    </div>

    <div v-loading="pluginStore.loading">
      <el-empty v-if="pluginStore.plugins.length === 0" description="还没有插件" />
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
                <el-tag v-if="p.is_builtin" size="small" type="info">内置</el-tag>
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
              <el-button
                v-if="!p.is_builtin"
                text
                size="small"
                type="danger"
                @click="onDelete(p)"
              >
                删除
              </el-button>
            </div>
          </div>
        </el-card>
      </div>
    </div>

    <!-- 添加插件弹窗 -->
    <el-dialog v-model="showAdd" title="添加自定义插件" width="90%" top="3vh" style="max-width: 1200px;">
      <el-form :model="form" label-width="140px" size="small">
        <el-form-item label="插件 ID">
          <el-input v-model="form.id" placeholder="如: my-app (英文, 用于文件名)" />
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="form.name" placeholder="如: My App" />
        </el-form-item>
        <el-form-item label="图标 (emoji)">
          <el-input v-model="form.icon" placeholder="如: 🎮" style="width: 80px" />
        </el-form-item>
        <el-divider>路径配置</el-divider>
        <el-form-item label="进程名">
          <el-input v-model="form.process_names" placeholder="如: MyApp.exe (多个用逗号分隔)" />
        </el-form-item>
        <el-form-item label="exe 候选路径">
          <el-input
            v-model="form.exe_candidates"
            type="textarea"
            :rows="2"
            placeholder="每行一个, 支持 %LOCALAPPDATA% %PROGRAMFILES% 等环境变量"
          />
        </el-form-item>
        <el-form-item label="配置目录">
          <el-input v-model="form.config_dir" placeholder="如: %APPDATA%/MyApp" />
        </el-form-item>
        <el-form-item label="用户数据目录">
          <el-input v-model="form.user_dir" placeholder="如: %USERPROFILE%/.myapp (可选)" />
        </el-form-item>
        <el-form-item label="跳过项">
          <el-input v-model="form.skip_items" placeholder="如: Cache, GPUCache, logs (逗号分隔)" />
        </el-form-item>
        <el-divider>机器码 (可选)</el-divider>
        <el-form-item label="机器码文件路径">
          <el-input v-model="form.machine_id_path" placeholder="如: %APPDATA%/MyApp/machineid (留空则不管理机器码)" />
        </el-form-item>
        <el-divider>登录痕迹 (可选)</el-divider>
        <el-form-item label="登录痕迹路径">
          <el-input
            v-model="form.login_artifacts"
            type="textarea"
            :rows="2"
            placeholder="每行一个文件/目录路径, 支持 dir: 或 file: 前缀"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showAdd = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onAdd">创建</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { Plus } from "@element-plus/icons-vue";
import { usePluginStore } from "@/stores/plugin";
import type { PluginInfo } from "@/types";

const pluginStore = usePluginStore();
const showAdd = ref(false);
const saving = ref(false);

const form = reactive({
  id: "",
  name: "",
  icon: "📦",
  process_names: "",
  exe_candidates: "",
  config_dir: "",
  user_dir: "",
  skip_items: "Cache, GPUCache, logs",
  machine_id_path: "",
  login_artifacts: "",
});

function resetForm() {
  Object.assign(form, {
    id: "",
    name: "",
    icon: "📦",
    process_names: "",
    exe_candidates: "",
    config_dir: "",
    user_dir: "",
    skip_items: "Cache, GPUCache, logs",
    machine_id_path: "",
    login_artifacts: "",
  });
}

async function onToggle(pluginId: string, enabled: boolean) {
  try {
    await pluginStore.toggle(pluginId, enabled);
    ElMessage.success(enabled ? "已启用" : "已禁用");
  } catch (e) {
    ElMessage.error("操作失败: " + e);
  }
}

async function onDelete(p: PluginInfo) {
  try {
    await ElMessageBox.confirm(`确定删除插件「${p.name}」？`, "删除确认", { type: "warning" });
    await pluginStore.remove(p.id);
    ElMessage.success("已删除");
  } catch {
    // cancel
  }
}

async function onAdd() {
  if (!form.id || !form.name || !form.config_dir) {
    ElMessage.warning("请至少填写 ID、名称、配置目录");
    return;
  }
  saving.value = true;
  try {
    const processNames = form.process_names.split(",").map((s) => s.trim()).filter(Boolean);
    const exeCandidates = form.exe_candidates.split("\n").map((s) => s.trim()).filter(Boolean);
    const skipItems = form.skip_items.split(",").map((s) => s.trim()).filter(Boolean);
    const loginArtifacts = form.login_artifacts
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean)
      .map((s) => {
        const isDir = s.startsWith("dir:");
        const path = s.replace(/^(dir:|file:)/, "");
        return { type: isDir ? "dir" : "file", path };
      });

    const config: Record<string, unknown> = {
      id: form.id,
      name: form.name,
      version: "0.1.0",
      icon: form.icon,
      homepage: "",
      process_names: processNames,
      exe_candidates: exeCandidates,
      data_dirs: [
        {
          path: form.config_dir,
          label: "配置目录",
          include_subdirs: [] as string[],
        },
        ...(form.user_dir
          ? [{ path: form.user_dir, label: "用户数据目录", include_subdirs: [] as string[] }]
          : []),
      ],
      skip_items: skipItems,
      machine_id: form.machine_id_path
        ? { type: "file", path: form.machine_id_path, label: "机器码" }
        : { type: "file", path: "", label: "" },
      login_artifacts: loginArtifacts,
      switch_flow: [
        { type: "ensure_not_running_or_kill", timeout: 5000 },
        { type: "backup_current" },
        { type: "restore_snapshot" },
        ...(form.machine_id_path ? [{ type: "write_machine_id" }] : []),
      ],
      clear_login_flow: [
        { type: "ensure_not_running_or_kill", timeout: 5000 },
        { type: "delete_login_artifacts" },
        ...(form.machine_id_path ? [{ type: "reset_machine_id" }] : []),
      ],
    };

    await pluginStore.add(config);
    ElMessage.success("插件已添加");
    showAdd.value = false;
    resetForm();
  } catch (e) {
    ElMessage.error("添加失败: " + e);
  } finally {
    saving.value = false;
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
