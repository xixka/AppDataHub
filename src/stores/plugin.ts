/**
 * 插件 Store
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import * as api from "@/api";
import type { PluginInfo } from "@/types";

export const usePluginStore = defineStore("plugins", () => {
  const plugins = ref<PluginInfo[]>([]);
  const activePluginId = ref<string>("");
  const loading = ref(false);

  const activePlugin = computed(
    () => plugins.value.find((p) => p.id === activePluginId.value) ?? null,
  );

  async function load() {
    loading.value = true;
    try {
      plugins.value = await api.listPlugins();
      if (!activePluginId.value && plugins.value.length > 0) {
        activePluginId.value = plugins.value[0].id;
      }
    } finally {
      loading.value = false;
    }
  }

  async function reload() {
    await api.reloadPlugins();
    await load();
  }

  function selectPlugin(id: string) {
    activePluginId.value = id;
  }

  async function toggle(pluginId: string, enabled: boolean) {
    if (enabled) {
      await api.enablePlugin(pluginId);
    } else {
      await api.disablePlugin(pluginId);
    }
    await load();
  }

  return {
    plugins,
    activePluginId,
    activePlugin,
    loading,
    load,
    reload,
    selectPlugin,
    toggle,
  };
});
