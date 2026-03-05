<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

const isRecording = ref(false);
let unlisten: UnlistenFn;

onMounted(async () => {
  isRecording.value = await invoke('get_recording_state');
  unlisten = await listen<boolean>('recording-state-changed', (e) => {
    isRecording.value = e.payload;
  });
});

onUnmounted(() => unlisten?.());

const toggleRecording = () => invoke('set_recording', { recording: !isRecording.value });
const openSettings = () => invoke('open_settings');
const closeWindow = () => invoke('hide_and_stop_recording');
</script>

<template>
  <div class="apple-theme">
    <div class="titlebar" data-tauri-drag-region>
      <div class="drag-handle"></div>
      <button class="close-btn" @click="closeWindow">×</button>
    </div>
    <div class="content">
      <button class="mic-btn" :class="{ active: isRecording }" @click="toggleRecording">
        {{ isRecording ? '🔴 录音中' : '🎤 点击录音' }}
      </button>
      <button class="settings-btn" @click="openSettings">设置</button>
    </div>
  </div>
</template>

<style scoped>
.apple-theme { width: 100%; height: 100%; background: linear-gradient(180deg, #f5f5f7 0%, #e8e8ed 100%); border-radius: 20px; display: flex; flex-direction: column; }
.titlebar { height: 32px; display: flex; align-items: center; justify-content: space-between; padding: 0 12px; background: rgba(255, 255, 255, 0.7); backdrop-filter: blur(10px); border-radius: 20px 20px 0 0; -webkit-app-region: drag; }
.drag-handle { width: 40px; height: 5px; background: rgba(0, 0, 0, 0.2); border-radius: 3px; }
.close-btn { width: 24px; height: 24px; border: none; background: rgba(255, 59, 48, 0.9); color: white; border-radius: 50%; cursor: pointer; -webkit-app-region: no-drag; }
.content { flex: 1; padding: 20px; display: flex; flex-direction: column; gap: 12px; align-items: center; justify-content: center; }
.mic-btn { padding: 16px 32px; border-radius: 16px; border: none; background: #007aff; color: white; cursor: pointer; font-family: -apple-system, BlinkMacSystemFont, sans-serif; font-size: 16px; transition: all 0.2s; }
.mic-btn.active { background: #ff3b30; }
.settings-btn { padding: 12px 24px; border-radius: 12px; border: none; background: rgba(0, 0, 0, 0.1); color: #1d1d1f; cursor: pointer; font-family: -apple-system, BlinkMacSystemFont, sans-serif; }
</style>
