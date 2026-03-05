<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import AsrSettings, { type ASRConfig } from './settings/asr/AsrSettings.vue';

// 类型定义
interface AppConfig {
  shortcut: string;
  auto_start: boolean;
  asr: ASRConfig;
}

// 响应式数据
const config = ref<AppConfig>({
  shortcut: 'Shift+E',
  auto_start: false,
  asr: {
    provider: 'doubao',
    doubao: { api_key: '' },
    xunfei: { app_id: '', api_key: '', api_secret: '' },
  }
});

const isRecording = ref(false);
const msg = ref('');
let timeout: number;

// 计算属性
const displayShortcut = computed(() => 
  isRecording.value ? '按下快捷键...' : config.value.shortcut.replace(/\+/g, ' + ')
);

const asrConfig = computed({
  get: () => config.value.asr,
  set: (val) => { config.value.asr = val; }
});

// 生命周期
onMounted(async () => {
  try {
    const loaded = await invoke<AppConfig>('get_config');
    config.value = { ...config.value, ...loaded, asr: { ...config.value.asr, ...loaded.asr } };
  } catch (e) {
    console.error(e);
  }
});

// 窗口控制
const closeWindow = () => invoke('close_settings_window');

// 配置持久化
const save = async () => {
  try {
    await invoke('sync_config', { newConfig: config.value });
    showMsg('已保存');
  } catch (e) {
    showMsg('保存失败');
  }
};

const showMsg = (text: string, time = 1500) => {
  msg.value = text;
  clearTimeout(timeout);
  timeout = setTimeout(() => msg.value = '', time);
};

// 快捷键录制
const onKey = (e: KeyboardEvent) => {
  if (!isRecording.value) return;
  e.preventDefault();
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;
  
  const mods = [];
  if (e.ctrlKey) mods.push('Ctrl');
  if (e.altKey) mods.push('Alt');
  if (e.shiftKey) mods.push('Shift');
  if (e.metaKey) mods.push('Cmd');
  
  const key = e.key === ' ' ? 'Space' : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  
  if (!mods.length) return showMsg('需要修饰键');
  
  config.value.shortcut = [...mods, key].join('+');
  isRecording.value = false;
  save();
};
</script>

<template>
  <div class="settings" tabindex="0" @keydown="onKey">
    <!-- 标题栏 -->
    <div class="title-bar" data-tauri-drag-region>
      <div class="drag-handle"></div>
      <button class="close-btn" @click="closeWindow">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <path d="M18 6L6 18M6 6l12 12"/>
        </svg>
      </button>
    </div>

    <!-- 快捷键 -->
    <div class="item">
      <div>
        <div class="label">快捷键</div>
        <div class="desc">开始语音输入</div>
      </div>
      <div class="shortcut" :class="{ recording: isRecording }" @click="isRecording = true">
        {{ displayShortcut }}
      </div>
    </div>

    <!-- 开机自启 -->
    <div class="item">
      <div class="label">开机自动启动</div>
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
/* 布局 */
.settings {
  width: 100%;
  height: 100%;
  background: #efefef;
  border-radius: 8px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  padding: 0 20px 20px;
}

.settings::-webkit-scrollbar {
  display: none;
}

/* 标题栏 */
.title-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 28px;
  margin: 0 -20px;
  flex-shrink: 0;
  -webkit-app-region: drag;
  app-region: drag;
  position: relative;
}

.drag-handle {
  width: 32px;
  height: 4px;
  border-radius: 2px;
  background: rgba(0, 0, 0, 0.15);
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.close-btn {
  position: absolute;
  right: 8px;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: #5f6368;
  cursor: pointer;
  transition: all 0.2s;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.close-btn:hover {
  background: rgba(0, 0, 0, 0.06);
  color: #333;
}

.close-btn svg {
  width: 12px;
  height: 12px;
}

/* 设置项通用样式 */
.item {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.label {
  font-size: 13px;
  color: #202124;
}

.desc {
  font-size: 11px;
  color: #5f6368;
  margin-top: 2px;
}

/* 快捷键输入 */
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

/* 开关组件 */
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

/* 提示消息 */
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

/* 动画 */
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateX(-50%) translateY(10px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}
</style>
