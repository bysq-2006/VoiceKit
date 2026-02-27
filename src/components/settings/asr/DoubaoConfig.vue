<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import PasswordInput from '../../PasswordInput.vue';

interface DoubaoConfigData {
  app_id?: string;
  api_key?: string;
}

const props = defineProps<{
  modelValue: DoubaoConfigData;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: DoubaoConfigData];
  save: [];
}>();

const testing = ref(false);
const msg = ref('');
let timeout: number;

const updateField = <K extends keyof DoubaoConfigData>(field: K, value: DoubaoConfigData[K]) => {
  emit('update:modelValue', { ...props.modelValue, [field]: value });
};

const showMsg = (text: string, time = 1500) => {
  msg.value = text;
  clearTimeout(timeout);
  timeout = setTimeout(() => msg.value = '', time);
};

const testConnection = async () => {
  const { app_id, api_key } = props.modelValue;
  if (!app_id || !api_key) {
    return showMsg('请填写 App ID 和 API Key', 2000);
  }
  testing.value = true;
  showMsg('测试中...', 5000);
  try {
    await invoke('test_asr_config', {
      config: {
        provider: 'doubao',
        doubao: props.modelValue,
        xunfei: {},
      }
    });
    showMsg('连接成功！', 2000);
  } catch (e: any) {
    showMsg(e || '连接失败', 3000);
  } finally {
    testing.value = false;
  }
};
</script>

<template>
  <div class="asr-config">
    <input
      :value="modelValue.app_id"
      @input="e => updateField('app_id', (e.target as HTMLInputElement).value)"
      @blur="$emit('save')"
      placeholder="App ID"
    />
    <PasswordInput
      :modelValue="modelValue.api_key || ''"
      @update:modelValue="value => updateField('api_key', value)"
      @blur="$emit('save')"
      placeholder="API Key"
    />
    <div class="hint">
      <span>豆包语音识别服务（火山引擎）</span>
    </div>
    <div class="actions">
      <button @click="testConnection" :disabled="testing">
        {{ testing ? '测试中...' : '测试连接' }}
      </button>
    </div>
    <div v-if="msg" class="toast">{{ msg }}</div>
  </div>
</template>

<style scoped>
.asr-config {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

input {
  padding: 8px 12px;
  border: 1px solid #dadce0;
  border-radius: 4px;
  font-size: 13px;
  background: white;
  width: 100%;
  box-sizing: border-box;
}

input:focus {
  outline: none;
  border-color: #0d9488;
}

.hint {
  font-size: 11px;
  color: #5f6368;
  padding: 0 4px;
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

@keyframes fadeIn {
  from { opacity: 0; transform: translateX(-50%) translateY(10px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}
</style>
