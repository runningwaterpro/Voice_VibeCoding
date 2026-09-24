import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { VoiceSnapshot } from "../types";

export const useVoiceStatusStore = defineStore("voiceStatus", () => {
  const snapshot = ref<VoiceSnapshot | null>(null);
  let unlisten: UnlistenFn | null = null;

  async function refresh() {
    try {
      snapshot.value = await invoke<VoiceSnapshot>("get_voice_snapshot");
    } catch (error) {
      console.warn("voice snapshot refresh failed", error);
    }
  }

  async function init() {
    if (!unlisten) {
      unlisten = await listen<VoiceSnapshot>("voice-snapshot", (event) => {
        snapshot.value = event.payload;
      });
    }
    await refresh();
  }

  function dispose() {
    unlisten?.();
    unlisten = null;
  }

  return { snapshot, refresh, init, dispose };
});
