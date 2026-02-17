<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getConfig, syncConfig } from '../api/config';
import type { AppConfig } from '../api/config';

const config = ref<AppConfig>({
  api_key: null,
  shortcut: 'Shift+E',
  theme: 'system',
  auto_start: false,
});
const isRecording = ref(false);
const displayShortcut = ref('');
const message = ref('');

onMounted(async () => {
  try {
    const loaded = await getConfig();
    if (loaded) {
      config.value = loaded;
    }
  } catch (e) {
    console.error('加载配置失败:', e);
  }
  displayShortcut.value = formatShortcutDisplay(config.value.shortcut);
});

// 格式化快捷键显示
function formatShortcutDisplay(shortcut: string): string {
  return shortcut.replace(/\+/g, ' + ');
}

// 保存配置
async function saveConfig() {
  try {
    await syncConfig(config.value);
    message.value = '已保存';
    setTimeout(() => message.value = '', 1500);
  } catch (e) {
    message.value = '保存失败';
  }
}

// 开始录制快捷键
function startRecording() {
  isRecording.value = true;
  displayShortcut.value = '按下快捷键...';
}

// 处理键盘事件
async function handleKeyDown(event: KeyboardEvent) {
  if (!isRecording.value) return;
  
  event.preventDefault();
  event.stopPropagation();
  
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) {
    return;
  }
  
  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push('Ctrl');
  if (event.altKey) modifiers.push('Alt');
  if (event.shiftKey) modifiers.push('Shift');
  if (event.metaKey) modifiers.push('Cmd');
  
  let key = event.key;
  if (key === ' ') {
    key = 'Space';
  } else if (key.length === 1) {
    key = key.toUpperCase();
  }
  
  if (modifiers.length === 0) {
    displayShortcut.value = '需要包含修饰键';
    setTimeout(() => {
      if (isRecording.value) {
        displayShortcut.value = '按下快捷键...';
      }
    }, 1500);
    return;
  }
  
  const shortcut = [...modifiers, key].join('+');
  
  config.value.shortcut = shortcut;
  displayShortcut.value = formatShortcutDisplay(shortcut);
  isRecording.value = false;
  
  // 自动保存快捷键
  await saveConfig();
}

// 打开外部链接
async function openLink(url: string) {
  await invoke('open_link', { url });
}
</script>

<template>
  <div class="settings-container">
    <!-- 快捷键设置 -->
    <div class="setting-item">
      <div class="setting-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
          <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
        </svg>
      </div>
      <div class="setting-content">
        <div class="setting-title">快捷键</div>
        <div class="setting-desc">按下快捷键开始语音输入</div>
      </div>
      <div 
        class="shortcut-box"
        :class="{ recording: isRecording }"
        @click="startRecording"
        @keydown="handleKeyDown"
        tabindex="0"
      >
        {{ displayShortcut }}
      </div>
    </div>

    <!-- 分隔线 -->
    <div class="divider"></div>

    <!-- 开机自启 -->
    <div class="setting-item">
      <div class="setting-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"></path>
        </svg>
      </div>
      <div class="setting-content">
        <div class="setting-title">开机自动启动</div>
      </div>
      <label class="switch">
        <input type="checkbox" v-model="config.auto_start" @change="saveConfig">
        <span class="slider"></span>
      </label>
    </div>

    <!-- 分隔线 -->
    <div class="divider"></div>

    <!-- 关于/反馈 -->
    <div class="setting-item link-item" @click="openLink('https://github.com/your-repo/issues')">
      <div class="setting-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"></circle>
          <path d="M12 16v-4M12 8h.01"></path>
        </svg>
      </div>
      <div class="setting-content">
        <div class="setting-title">反馈问题</div>
      </div>
      <div class="arrow">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="9 18 15 12 9 6"></polyline>
        </svg>
      </div>
    </div>

    <!-- 保存成功提示 -->
    <div v-if="message" class="toast">{{ message }}</div>
  </div>
</template>

<style scoped>
.settings-container {
  width: 100%;
  height: 100%;
  background: #efefef;
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  padding: 8px 0;
}

.setting-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  gap: 12px;
  transition: background 0.2s;
}

.setting-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.setting-icon {
  width: 20px;
  height: 20px;
  color: #5f6368;
  flex-shrink: 0;
}

.setting-icon svg {
  width: 100%;
  height: 100%;
}

.setting-content {
  flex: 1;
  min-width: 0;
}

.setting-title {
  font-size: 13px;
  color: #202124;
  line-height: 1.4;
}

.setting-desc {
  font-size: 11px;
  color: #5f6368;
  margin-top: 2px;
}

/* 快捷键输入框 */
.shortcut-box {
  padding: 4px 10px;
  background: white;
  border: 1px solid #dadce0;
  border-radius: 4px;
  font-size: 12px;
  color: #202124;
  min-width: 80px;
  text-align: center;
  cursor: pointer;
  user-select: none;
  transition: all 0.2s;
}

.shortcut-box:hover {
  border-color: #bdc1c6;
}

.shortcut-box.recording {
  border-color: #0d9488;
  background: #e6f7ff;
  animation: pulse 1.5s infinite;
}

.shortcut-box:focus {
  outline: none;
  border-color: #0d9488;
}

/* 开关 */
.switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
  flex-shrink: 0;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: #ccc;
  transition: 0.2s;
  border-radius: 20px;
}

.slider:before {
  position: absolute;
  content: "";
  height: 16px;
  width: 16px;
  left: 2px;
  bottom: 2px;
  background-color: white;
  transition: 0.2s;
  border-radius: 50%;
  box-shadow: 0 1px 3px rgba(0,0,0,0.2);
}

input:checked + .slider {
  background-color: #0d9488;
}

input:checked + .slider:before {
  transform: translateX(16px);
}

/* 分隔线 */
.divider {
  height: 1px;
  background: #dadce0;
  margin: 8px 0;
}

/* 可点击项 */
.link-item {
  cursor: pointer;
}

.arrow {
  width: 16px;
  height: 16px;
  color: #5f6368;
}

.arrow svg {
  width: 100%;
  height: 100%;
}

/* Toast 提示 */
.toast {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.8);
  color: white;
  padding: 8px 16px;
  border-radius: 4px;
  font-size: 13px;
  animation: fadeInOut 2s ease;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

@keyframes fadeInOut {
  0% { opacity: 0; transform: translateX(-50%) translateY(10px); }
  20%, 80% { opacity: 1; transform: translateX(-50%) translateY(0); }
  100% { opacity: 0; transform: translateX(-50%) translateY(-10px); }
}
</style>
