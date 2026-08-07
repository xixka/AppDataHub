/**
 * 设置 Store
 */
import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "@/api";
import type { AppSettings } from "@/types";

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<AppSettings>({});
  const loaded = ref(false);

  async function load() {
    settings.value = await api.getSettings();
    loaded.value = true;
  }

  async function save(newSettings: AppSettings) {
    await api.updateSettings(newSettings);
    settings.value = newSettings;
  }

  return { settings, loaded, load, save };
});
