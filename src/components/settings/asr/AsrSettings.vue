<script setup lang="ts">
import { computed } from 'vue';
import DoubaoConfig from './DoubaoConfig.vue';
import XunfeiConfig from './XunfeiConfig.vue';

export type ASRProviderType = 'doubao' | 'xunfei';

// 豆包配置
export interface DoubaoConfigData {
  api_key?: string;
}

// 讯飞配置
export interface XunfeiConfigData {
  app_id?: string;
  api_key?: string;
  api_secret?: string;
}

// ASR 总配置（与后端结构一致）
export interface ASRConfig {
  provider: ASRProviderType;
  doubao: DoubaoConfigData;
  xunfei: XunfeiConfigData;
}

const props = defineProps<{
  modelValue: ASRConfig;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: ASRConfig];
  save: [];
}>();

const providers = [
  { key: 'doubao' as ASRProviderType, name: '豆包 (Volcengine)' },
  { key: 'xunfei' as ASRProviderType, name: '讯飞 (iFlytek)' },
];

const currentProvider = computed({
  get: () => props.modelValue.provider,
  set: (val: ASRProviderType) => {
    // 只切换 provider，保留所有服务商的配置
    emit('update:modelValue', { ...props.modelValue, provider: val });
    emit('save');
  }
});

const doubaoConfig = computed({
  get: () => props.modelValue.doubao,
  set: (val) => {
    emit('update:modelValue', {
      ...props.modelValue,
      doubao: val,
    });
  }
});

const xunfeiConfig = computed({
  get: () => props.modelValue.xunfei,
  set: (val) => {
    emit('update:modelValue', {
      ...props.modelValue,
      xunfei: val,
    });
  }
});
</script>

<template>
  <div class="asr-settings">
    <!-- ASR 提供商选择 -->
    <div class="item">
      <div>
        <div class="title">语音识别服务</div>
      </div>
      <select v-model="currentProvider">
        <option v-for="p in providers" :key="p.key" :value="p.key">{{ p.name }}</option>
      </select>
    </div>

    <!-- 动态配置区 -->
    <div class="config-area">
      <DoubaoConfig
        v-if="modelValue.provider === 'doubao'"
        v-model="doubaoConfig"
        @save="$emit('save')"
      />
      <XunfeiConfig
        v-else-if="modelValue.provider === 'xunfei'"
        v-model="xunfeiConfig"
        @save="$emit('save')"
      />
    </div>
  </div>
</template>

<style scoped>
.asr-settings {
  display: flex;
  flex-direction: column;
  gap: 16px;
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

select {
  padding: 8px 12px;
  border: 1px solid #dadce0;
  border-radius: 4px;
  font-size: 13px;
  background: white;
  width: 180px;
}

select:focus {
  outline: none;
  border-color: #0d9488;
}

.config-area {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
</style>
