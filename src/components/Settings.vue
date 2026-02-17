<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface ASRConfig {
  provider: 'doubao';
  app_id?: string;
  access_key?: string;
  resource_id?: string;
}

interface AppConfig {
  shortcut: string;
  auto_start: boolean;
  asr: ASRConfig;
}

const config = ref<AppConfig>({
  shortcut: 'Shift+E',
  auto_start: false,
  asr: { provider: 'doubao', resource_id: 'volc.seedasr.sauc.concurrent' }
});

const isRecording = ref(false);
const msg = ref('');
const testing = ref(false);
let timeout: number;

const providers = [
  { key: 'doubao', name: '豆包 (Volcengine)' }
];

const resources = [
  { key: 'volc.seedasr.sauc.concurrent', name: '豆包 2.0 - 并发版' },
  { key: 'volc.seedasr.sauc.duration', name: '豆包 2.0 - 小时版' },
  { key: 'volc.bigasr.sauc.concurrent', name: '豆包 1.0 - 并发版' },
  { key: 'volc.bigasr.sauc.duration', name: '豆包 1.0 - 小时版' },
];

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

const testASR = async () => {
  const { asr } = config.value;
  if (!asr.app_id || !asr.access_key) {
    return showMsg('请填写 App ID 和 Access Key', 2000);
  }
  testing.value = true;
  showMsg('测试中...', 5000);
  try {
    await invoke('test_asr_config', { config: asr });
    showMsg('连接成功！', 2000);
  } catch (e: any) {
    showMsg(e || '连接失败', 3000);
  } finally {
    testing.value = false;
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

    <div class="divider"></div>

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

    <div class="divider"></div>

    <!-- ASR 提供商选择 -->
    <div class="item">
      <div>
        <div class="title">语音识别服务</div>
      </div>
      <select v-model="config.asr.provider" @change="save">
        <option v-for="p in providers" :key="p.key" :value="p.key">{{ p.name }}</option>
      </select>
    </div>

    <!-- 动态配置区 -->
    <div class="config-area">
      <template v-if="config.asr.provider === 'doubao'">
        <input v-model="config.asr.app_id" placeholder="App ID" @blur="save" />
        <input v-model="config.asr.access_key" type="password" placeholder="Access Key" @blur="save" />
        <select v-model="config.asr.resource_id" @change="save">
          <option v-for="r in resources" :key="r.key" :value="r.key">{{ r.name }}</option>
        </select>
        <div class="actions">
          <button @click="testASR" :disabled="testing">{{ testing ? '测试中...' : '测试连接' }}</button>
        </div>
      </template>
    </div>

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
  overflow-y: auto;
  padding: 16px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  box-sizing: border-box;
}

.item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
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

.divider {
  height: 1px;
  background: #dadce0;
  margin: 8px 0;
}

select, input {
  padding: 8px 12px;
  border: 1px solid #dadce0;
  border-radius: 4px;
  font-size: 13px;
  background: white;
  width: 180px;
}

select:focus, input:focus {
  outline: none;
  border-color: #0d9488;
}

.config-area {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 0;
}

.config-area input, .config-area select {
  width: 100%;
  box-sizing: border-box;
}

.actions {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}

button {
  flex: 1;
  padding: 8px;
  border: none;
  border-radius: 4px;
  background: #0d9488;
  color: white;
  font-size: 13px;
  cursor: pointer;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
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
