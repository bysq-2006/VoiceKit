<script setup lang="ts">
import { computed } from 'vue';
import DoubaoConfig from './DoubaoConfig.vue';
import XunfeiConfig from './XunfeiConfig.vue';

export type ASRProviderType = 'doubao' | 'xunfei';

export interface ASRConfig {
  provider: ASRProviderType;
  // 通用字段（与后端一致）
  api_id?: string;        // App ID / Access Key ID
  api_key?: string;       // Access Key / API Key
  api_secret?: string;    // API Secret（讯飞需要）
  // 豆包特有
  resource_id?: string;
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
    emit('update:modelValue', { ...props.modelValue, provider: val });
    emit('save');
  }
});

const doubaoConfig = computed({
  get: () => ({
    api_id: props.modelValue.api_id,
    api_key: props.modelValue.api_key,
    resource_id: props.modelValue.resource_id,
  }),
  set: (val) => {
    emit('update:modelValue', {
      ...props.modelValue,
      api_id: val.api_id,
      api_key: val.api_key,
      resource_id: val.resource_id,
    });
  }
});

const xunfeiConfig = computed({
  get: () => ({
    api_id: props.modelValue.api_id,
    api_key: props.modelValue.api_key,
    api_secret: props.modelValue.api_secret,
  }),
  set: (val) => {
    emit('update:modelValue', {
      ...props.modelValue,
      api_id: val.api_id,
      api_key: val.api_key,
      api_secret: val.api_secret,
    });
  }
});

const onProviderChange = () => {
  // 切换提供商时，清空特定配置避免混淆
  const newConfig: ASRConfig = {
    provider: currentProvider.value,
  };
  emit('update:modelValue', newConfig);
  emit('save');
};
</script>

<template>
  <div class="asr-settings">
    <!-- ASR 提供商选择 -->
    <div class="item">
      <div>
        <div class="title">语音识别服务</div>
      </div>
      <select v-model="currentProvider" @change="onProviderChange">
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
