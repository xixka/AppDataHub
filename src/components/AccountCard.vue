<template>
  <el-card class="account-card" :class="{ active: account.is_active }" shadow="hover">
    <div class="card-body">
      <div class="card-left">
        <el-avatar :size="40" :src="avatarUrl">
          {{ account.name.charAt(0).toUpperCase() }}
        </el-avatar>
      </div>
      <div class="card-center">
        <div class="card-name">
          {{ account.name }}
          <el-tag v-if="account.is_active" size="small" type="success">当前</el-tag>
        </div>
        <div class="card-meta">
          <span v-if="account.email">{{ account.email }}</span>
          <span v-if="account.has_snapshot" class="dot">有快照</span>
          <span v-else class="dot warn">无快照</span>
        </div>
      </div>
      <div class="card-right">
        <el-button
          type="primary"
          size="small"
          :disabled="account.is_active"
          :loading="accountStore.switching"
          @click="$emit('switch', account.id)"
        >
          切换
        </el-button>
        <el-button size="small" @click="$emit('save', account.id)">
          {{ account.has_snapshot ? "更新快照" : "保存快照" }}
        </el-button>
        <el-button text size="small" @click="$emit('edit', account)">编辑</el-button>
        <el-button text size="small" type="danger" @click="$emit('delete', account.id)">删除</el-button>
      </div>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useAccountStore } from "@/stores/account";
import type { AccountMetadata } from "@/types";

const props = defineProps<{ account: AccountMetadata }>();
defineEmits<{
  switch: [id: string];
  save: [id: string];
  edit: [account: AccountMetadata];
  delete: [id: string];
}>();

const accountStore = useAccountStore();

const avatarUrl = computed(() => {
  const colors = ["#3b82f6", "#8b5cf6", "#14b8a6", "#f43f5e", "#d946ef"];
  const idx = props.account.name.charCodeAt(0) % colors.length;
  return "";
});

// Force re-evaluate color (unused but for type)
void avatarUrl;
</script>

<style scoped>
.account-card.active {
  border-color: var(--el-color-success);
}

.card-body {
  display: flex;
  align-items: center;
  gap: 16px;
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
  gap: 12px;
  margin-top: 4px;
}

.dot::before {
  content: "● ";
}
.dot.warn::before {
  color: var(--el-color-warning);
}

.card-right {
  display: flex;
  gap: 4px;
  align-items: center;
}
</style>
