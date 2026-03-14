<script setup lang="ts">
import { computed } from 'vue';
import DoubaoConfig from './DoubaoConfig.vue';
import FunasrConfig from './FunasrConfig.vue';
import XunfeiConfig from './XunfeiConfig.vue';
import type { ASRConfig } from '../../../composables/useConfig';

const props = defineProps<{
  modelValue: ASRConfig;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: ASRConfig];
  save: [];
}>();

const providers = [
  { key: 'doubao' as const, name: '豆包 (Volcengine)' },
  { key: 'xunfei' as const, name: '讯飞 (iFlytek)' },
  { key: 'funasr' as const, name: '本地 FunASR' },
];

const currentProvider = computed({
  get: () => props.modelValue.provider,
  set: (val) => {
    emit('update:modelValue', { ...props.modelValue, provider: val });
    emit('save');
  }
});

const doubaoConfig = computed({
  get: () => props.modelValue.doubao,
  set: (val) => {
    emit('update:modelValue', { ...props.modelValue, doubao: val });
  }
});

const xunfeiConfig = computed({
  get: () => props.modelValue.xunfei,
  set: (val) => {
    emit('update:modelValue', { ...props.modelValue, xunfei: val });
  }
});

const funasrConfig = computed({
  get: () => props.modelValue.funasr,
  set: (val) => {
    emit('update:modelValue', { ...props.modelValue, funasr: val });
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
      <FunasrConfig
        v-else-if="modelValue.provider === 'funasr'"
        v-model="funasrConfig"
        @save="$emit('save')"
      />
    </div>
  </div>
</template>

<style scoped>
.asr-settings {
  display: flex;
  flex-direction: column;
  gap: 12px;
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
  padding: 6px 12px;
  background: white;
  border: 1px solid #dadce0;
  border-radius: 4px;
  font-size: 12px;
  color: #202124;
  cursor: pointer;
  min-width: 140px;
}

select:focus {
  outline: none;
  border-color: #0d9488;
}

.config-area {
  margin-top: 8px;
}
</style>
