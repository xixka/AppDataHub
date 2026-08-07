<template>
  <div class="home">
    <div v-if="!pluginStore.activePlugin" class="empty-state">
      <el-empty description="请先在设置中配置插件路径" />
    </div>

    <template v-else>
      <!-- 顶部信息栏 -->
      <div class="header-bar">
        <div>
          <h2>{{ pluginStore.activePlugin.name }}</h2>
          <div class="header-meta">
            <el-tag size="small" :type="appRunning ? 'danger' : 'success'">
              {{ appRunning ? "运行中" : "未运行" }}
            </el-tag>
            <el-button text size="small" @click="checkRunning">刷新</el-button>
          </div>
        </div>
        <div class="header-actions">
          <el-button type="primary" @click="showAdd = true">
            <el-icon><Plus /></el-icon> 添加账号
          </el-button>
          <el-button @click="launchApp" :loading="launching">
            <el-icon><VideoPlay /></el-icon> 启动软件
          </el-button>
          <el-button type="warning" @click="regenerateDeviceId" :loading="regenerating">
            <el-icon><RefreshRight /></el-icon> 换新设备码
          </el-button>
          <el-button type="danger" @click="clearLogin">
            <el-icon><Delete /></el-icon> 清空数据
          </el-button>
        </div>
      </div>

      <!-- 账号列表 -->
      <div v-loading="accountStore.loading">
        <el-empty
          v-if="accountStore.accounts.length === 0"
          description="还没有账号，点击「添加账号」创建第一个"
        />
        <div v-else class="account-list">
          <AccountCard
            v-for="acc in accountStore.accounts"
            :key="acc.id"
            :account="acc"
            @switch="onSwitch"
            @save="onSave"
            @edit="onEdit"
            @delete="onDelete"
          />
        </div>
      </div>

      <!-- 添加/编辑对话框 -->
      <AddAccountDialog
        v-model:visible="showAdd"
        :plugin-id="pluginStore.activePluginId"
        :editing="editingAccount"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { Plus, VideoPlay, Delete, RefreshRight } from "@element-plus/icons-vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { usePluginStore } from "@/stores/plugin";
import { useAccountStore } from "@/stores/account";
import * as api from "@/api";
import type { AccountMetadata } from "@/types";
import AccountCard from "@/components/AccountCard.vue";
import AddAccountDialog from "@/components/AddAccountDialog.vue";

const pluginStore = usePluginStore();
const accountStore = useAccountStore();
const showAdd = ref(false);
const editingAccount = ref<AccountMetadata | null>(null);
const appRunning = ref(false);

async function loadAccounts() {
  if (pluginStore.activePluginId) {
    await accountStore.load(pluginStore.activePluginId);
    await checkRunning();
  }
}

async function checkRunning() {
  if (pluginStore.activePluginId) {
    appRunning.value = await api.checkAppRunning(pluginStore.activePluginId);
  }
}

const launching = ref(false);
const regenerating = ref(false);

async function launchApp() {
  if (!pluginStore.activePluginId) return;
  launching.value = true;
  try {
    await api.launchApp(pluginStore.activePluginId);
    ElMessage.success("已启动");
  } catch (e) {
    ElMessage.error("启动失败: " + e);
  } finally {
    launching.value = false;
  }
}

async function clearLogin() {
  try {
    await ElMessageBox.confirm(
      "清空账号数据将重置机器码并清除登录状态，确定继续？",
      "清空确认",
      { type: "warning" },
    );
    await accountStore.clearLogin(pluginStore.activePluginId);
    ElMessage.success("已清空");
    await loadAccounts();
  } catch (e) {
    if (e !== "cancel" && e !== undefined) {
      ElMessage.error("清空失败: " + e);
    }
  }
}

async function regenerateDeviceId() {
  if (!pluginStore.activePluginId) return;
  try {
    await ElMessageBox.confirm(
      "将生成全新的随机设备码并写入（用于重新签到领礼包）。请确保 TRAE SOLO CN 已关闭。确定继续？",
      "换新设备码",
      { type: "warning" },
    );
    regenerating.value = true;
    const newId = await api.regenerateMachineId(pluginStore.activePluginId);
    ElMessage.success(`已换新设备码 ${newId.slice(0, 8)}… 可重新签到`);
    await checkRunning();
  } catch (e) {
    if (e !== "cancel" && e !== undefined) {
      ElMessage.error("换码失败: " + e);
    }
  } finally {
    regenerating.value = false;
  }
}

async function onSwitch(id: string) {
  try {
    await ElMessageBox.confirm(
      "切换账号将备份当前配置并恢复目标账号的配置，确定继续？",
      "切换确认",
      { type: "warning" },
    );
    await accountStore.switchTo(id, pluginStore.activePluginId);
    ElMessage.success("切换成功");
    await checkRunning();
  } catch (e) {
    if (e !== "cancel" && e !== undefined) {
      ElMessage.error("切换失败: " + e);
    }
  }
}

function onEdit(acc: AccountMetadata) {
  editingAccount.value = acc;
  showAdd.value = true;
}

async function onDelete(id: string) {
  try {
    await ElMessageBox.confirm("确定删除此账号？快照数据也会清除", "删除确认", {
      type: "warning",
    });
    await accountStore.remove(id, pluginStore.activePluginId);
    ElMessage.success("已删除");
  } catch {
    // 用户取消
  }
}

watch(() => pluginStore.activePluginId, loadAccounts);
watch(() => showAdd.value, (v) => {
  if (!v) editingAccount.value = null;
});

onMounted(loadAccounts);
</script>

<style scoped>
.home {
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
  margin: 0 0 4px 0;
}

.header-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}

.account-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.empty-state {
  padding: 80px 0;
}
</style>
