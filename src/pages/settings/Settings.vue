<script setup lang="ts">
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useConfig } from '../../composables/useConfig';
import AsrSettings from './asr/AsrSettings.vue';
import { THEMES } from '../../themes/index';

const { config, update, save } = useConfig();
const themes = THEMES;

const isRecording = ref(false);
const msg = ref('');
let timeout: number;

const displayShortcut = computed(() =>
  isRecording.value ? '按下快捷键...' : config.value.shortcut.replace(/\+/g, ' + ')
);

const closeWindow = () => invoke('close_settings_window');

const showMsg = (text: string, time = 1500) => {
  msg.value = text;
  clearTimeout(timeout);
  timeout = setTimeout(() => msg.value = '', time);
};

const onKey = async (e: KeyboardEvent) => {
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
  
  isRecording.value = false;
  try {
    await update('shortcut', [...mods, key].join('+'));
    showMsg('已保存');
  } catch {
    showMsg('保存失败');
  }
};
</script>

<template>
  <div class="settings-wrapper">
    <!-- 标题栏 -->
    <div class="title-bar" data-tauri-drag-region>
      <div class="drag-handle"></div>
      <button class="close-btn" @click="closeWindow">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <path d="M18 6L6 18M6 6l12 12"/>
        </svg>
      </button>
    </div>

    <!-- 内容区域 -->
    <div class="settings-content" tabindex="0" @keydown="onKey">
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

    <!-- 主题选择 -->
    <div class="item">
      <div>
        <div class="label">界面主题</div>
        <div class="desc">选择主窗口的视觉风格</div>
      </div>
      <select v-model="config.theme" @change="save" class="theme-select">
        <option v-for="theme in themes" :key="theme.name" :value="theme.name">
          {{ theme.displayName }}
        </option>
      </select>
    </div>

    <!-- ASR 设置 -->
    <AsrSettings v-model="config.asr" @save="save" />

    <!-- 提示 -->
    <div v-if="msg" class="toast">{{ msg }}</div>
  </div>
  </div>
</template>

<style scoped>
/* 布局 */
.settings-wrapper {
  width: 100%;
  height: 100%;
  background: #efefef;
  border-radius: 8px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.settings-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  padding: 0 20px 20px;
  outline: none; /* 移除焦点黑线框 */
}

.settings-content::-webkit-scrollbar {
  display: none;
}

/* 标题栏 */
.title-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 28px;
  padding: 0 8px;
  flex-shrink: 0;
  -webkit-app-region: drag;
  app-region: drag;
  position: relative;
  /* 调试日志：确保整个标题栏都是可拖拽的 */
}

.drag-handle {
  width: 32px;
  height: 4px;
  border-radius: 2px;
  background: rgba(0, 0, 0, 0.15);
  pointer-events: none; /* 修复：让小条不拦截鼠标事件，使拖拽可以穿透 */
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

/* 主题选择下拉框 */
.theme-select {
  padding: 6px 12px;
  background: white;
  border: 1px solid #dadce0;
  border-radius: 4px;
  font-size: 12px;
  color: #202124;
  cursor: pointer;
  min-width: 120px;
}

.theme-select:focus {
  outline: none;
  border-color: #0d9488;
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
