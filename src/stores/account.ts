/**
 * 账号 Store
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import * as api from "@/api";
import type { AccountMetadata } from "@/types";

export const useAccountStore = defineStore("accounts", () => {
  const accounts = ref<AccountMetadata[]>([]);
  const loading = ref(false);
  const switching = ref(false);

  const activeAccount = computed(
    () => accounts.value.find((a) => a.is_active) ?? null,
  );

  async function load(pluginId: string) {
    loading.value = true;
    try {
      accounts.value = await api.listAccounts(pluginId);
    } finally {
      loading.value = false;
    }
  }

  async function add(params: {
    name: string;
    email: string | null;
    note: string | null;
    pluginId: string;
    machineId: string | null;
  }) {
    await api.addAccount(params);
    await load(params.pluginId);
  }

  async function update(params: {
    id: string;
    name: string;
    email: string | null;
    note: string | null;
    pluginId: string;
  }) {
    await api.updateAccount({
      id: params.id,
      name: params.name,
      email: params.email,
      note: params.note,
    });
    await load(params.pluginId);
  }

  async function remove(id: string, pluginId: string) {
    await api.deleteAccount(id);
    await load(pluginId);
  }

  async function saveSnapshot(accountId: string, pluginId: string) {
    await api.saveSnapshot(accountId);
    await load(pluginId);
  }

  async function switchTo(accountId: string, pluginId: string) {
    switching.value = true;
    try {
      await api.switchAccount(accountId);
      await load(pluginId);
    } finally {
      switching.value = false;
    }
  }

  async function clearLogin(pluginId: string) {
    await api.clearLoginState(pluginId);
    await load(pluginId);
  }

  return {
    accounts,
    loading,
    switching,
    activeAccount,
    load,
    add,
    update,
    remove,
    saveSnapshot,
    switchTo,
    clearLogin,
  };
});
