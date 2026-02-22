<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import AsrSettings, { type ASRConfig } from './settings/asr/AsrSettings.vue';

interface AppConfig {
  shortcut: string;
  auto_start: boolean;
  asr: ASRConfig;
}

const config = ref<AppConfig>({
  shortcut: 'Shift+E',
  auto_start: false,
  asr: { 
    provider: 'doubao', 
    api_id: '',
    api_key: '',
    resource_id: 'volc.seedasr.sauc.concurrent' 
  }
});

const isRecording = ref(false);
const msg = ref('');
let timeout: number;

onMounted(async () => {
  try {
    const loaded = await invoke<AppConfig>('get_config');
    config.value = { ...config.value, ...loaded, asr: { ...config.value.asr, ...loaded.asr } };
  } catch (e) {
    console.error(e);
  }
});

const showMsg = (text: string, time = 1500) => {
  msg.value = text;
  clearTimeout(timeout);
  timeout = setTimeout(() => msg.value = '', time);
};

const save = async () => {
  try {
    await invoke('sync_config', { newConfig: config.value });
    showMsg('已保存');
  } catch (e) {
    showMsg('保存失败');
  }
};

const recordShortcut = () => {
  isRecording.value = true;
};

const onKey = (e: KeyboardEvent) => {
  if (!isRecording.value) return;
  e.preventDefault();
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;
  
  const mods = [];
  if (e.ctrlKey) mods.push('Ctrl');
  if (e.altKey) mods.push('Alt');
  if (e.shiftKey) mods.push('Shift');
  if (e.metaKey) mods.push('Cmd');
  
  let key = e.key === ' ' ? 'Space' : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  
  if (!mods.length) {
    showMsg('需要修饰键');
    return;
  }
  
  config.value.shortcut = [...mods, key].join('+');
  isRecording.value = false;
  save();
};

const displayShortcut = computed(() => 
  isRecording.value ? '按下快捷键...' : config.value.shortcut.replace(/\+/g, ' + ')
);

const asrConfig = computed({
  get: () => config.value.asr,
  set: (val) => {
    config.value.asr = val;
  }
});
</script>

<template>
  <div class="settings" tabindex="0" @keydown="onKey">
    <!-- 快捷键 -->
    <div class="item">
      <div>
        <div class="title">快捷键</div>
        <div class="desc">开始语音输入</div>
      </div>
      <div class="shortcut" :class="{ recording: isRecording }" @click="recordShortcut">
        {{ displayShortcut }}
      </div>
    </div>

    <!-- 开机自启 -->
    <div class="item">
      <div>
        <div class="title">开机自动启动</div>
      </div>
      <label class="switch">
        <input type="checkbox" v-model="config.auto_start" @change="save">
        <span></span>
      </label>
    </div>

    <!-- ASR 设置 -->
    <AsrSettings v-model="asrConfig" @save="save" />

    <!-- 提示 -->
    <div v-if="msg" class="toast">{{ msg }}</div>
  </div>
</template>

<style scoped>
.settings {
  width: 100%;
  height: 100%;
  background: #efefef;
  border-radius: 8px;
  padding: 20px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.settings::-webkit-scrollbar {
  display: none;
}

.item {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 13px;
  color: #202124;
}

.desc {
  font-size: 11px;
  color: #5f6368;
  margin-top: 2px;
}

.shortcut {
  padding: 4px 12px;
  background: white;
  border: 1px solid #dadce0;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  min-width: 80px;
  text-align: center;
}

.shortcut.recording {
  border-color: #0d9488;
  background: #e6f7ff;
  animation: pulse 1.5s infinite;
}

.switch {
  position: relative;
  width: 36px;
  height: 20px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.switch span {
  position: absolute;
  inset: 0;
  background: #ccc;
  border-radius: 20px;
  cursor: pointer;
  transition: 0.2s;
}

.switch span::before {
  content: '';
  position: absolute;
  width: 16px;
  height: 16px;
  left: 2px;
  bottom: 2px;
  background: white;
  border-radius: 50%;
  transition: 0.2s;
  box-shadow: 0 1px 3px rgba(0,0,0,0.2);
}

.switch input:checked + span {
  background: #0d9488;
}

.switch input:checked + span::before {
  transform: translateX(16px);
}

.toast {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0,0,0,0.8);
  color: white;
  padding: 8px 16px;
  border-radius: 4px;
  font-size: 13px;
  animation: fadeIn 0.2s;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateX(-50%) translateY(10px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}
</style>
