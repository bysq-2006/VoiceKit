<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { getConfig, syncConfig } from '../api/config';
import type { AppConfig } from '../api/config';

const config = ref<AppConfig | null>(null);
const loading = ref(false);
const message = ref('');

// 支持的快捷键选项
const shortcutOptions = [
  { label: 'Shift + E', value: 'Shift+E' },
  { label: 'Ctrl + Space', value: 'Ctrl+Space' },
  { label: 'Alt + R', value: 'Alt+R' },
  { label: 'Ctrl + Shift + R', value: 'Ctrl+Shift+R' },
];

onMounted(async () => {
  config.value = await getConfig();
});

// 保存配置
async function saveConfig() {
  if (!config.value) return;
  
  loading.value = true;
  message.value = '';
  
  try {
    await syncConfig(config.value);
    message.value = '保存成功';
    setTimeout(() => message.value = '', 2000);
  } catch (e) {
    message.value = '保存失败';
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="settings-container">
    <h1>设置</h1>
    
    <div v-if="config" class="settings-form">
      <!-- 快捷键设置 -->
      <div class="setting-item">
        <label class="setting-label">
          语音转文字快捷键
          <span class="setting-desc">按下快捷键开始/停止录音</span>
        </label>
        <select v-model="config.shortcut" class="setting-input">
          <option v-for="opt in shortcutOptions" :key="opt.value" :value="opt.value">
            {{ opt.label }}
          </option>
        </select>
      </div>
      
      <!-- 开机自启 -->
      <div class="setting-item">
        <label class="setting-label checkbox-label">
          <input 
            type="checkbox" 
            v-model="config.auto_start"
            class="checkbox"
          />
          <span>开机自动启动</span>
        </label>
      </div>
      
      <!-- 保存按钮 -->
      <div class="setting-item">
        <button 
          @click="saveConfig" 
          :disabled="loading"
          class="save-btn"
        >
          {{ loading ? '保存中...' : '保存设置' }}
        </button>
        <span v-if="message" class="message">{{ message }}</span>
      </div>
    </div>
    
    <div v-else class="loading">加载中...</div>
  </div>
</template>

<style scoped>
.settings-container {
  width: 100%;
  height: 100vh;
  padding: 20px;
  background: #ffffff;
  box-sizing: border-box;
}

h1 {
  font-size: 18px;
  color: #333;
  margin: 0 0 24px 0;
  padding-bottom: 12px;
  border-bottom: 1px solid #e5e5e5;
}

.settings-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.setting-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.setting-label {
  font-size: 14px;
  color: #333;
  font-weight: 500;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.setting-desc {
  font-size: 12px;
  color: #999;
  font-weight: normal;
}

.checkbox-label {
  flex-direction: row;
  align-items: center;
  cursor: pointer;
}

.setting-input {
  padding: 8px 12px;
  border: 1px solid #d9d9d9;
  border-radius: 4px;
  font-size: 14px;
  background: #fff;
  cursor: pointer;
}

.setting-input:focus {
  outline: none;
  border-color: #40a9ff;
}

.checkbox {
  width: 16px;
  height: 16px;
  margin-right: 8px;
  cursor: pointer;
}

.save-btn {
  padding: 8px 20px;
  background: #1890ff;
  color: white;
  border: none;
  border-radius: 4px;
  font-size: 14px;
  cursor: pointer;
  width: fit-content;
}

.save-btn:hover {
  background: #40a9ff;
}

.save-btn:disabled {
  background: #bfbfbf;
  cursor: not-allowed;
}

.message {
  font-size: 13px;
  color: #52c41a;
  margin-left: 12px;
}

.loading {
  color: #999;
  font-size: 14px;
  padding: 40px 0;
  text-align: center;
}
</style>
