<script setup lang="ts">
import { ref, computed } from 'vue';

const props = defineProps<{
  modelValue: string;
  placeholder?: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
  blur: [];
}>();

const showPassword = ref(false);

const inputType = computed(() => showPassword.value ? 'text' : 'password');

const togglePassword = () => {
  showPassword.value = !showPassword.value;
};

const onInput = (e: Event) => {
  emit('update:modelValue', (e.target as HTMLInputElement).value);
};
</script>

<template>
  <div class="password-input-wrapper">
    <input
      :value="modelValue"
      @input="onInput"
      @blur="$emit('blur')"
      :type="inputType"
      :placeholder="placeholder"
      class="password-input"
    />
    <button
      type="button"
      class="toggle-btn"
      @click="togglePassword"
      :title="showPassword ? '隐藏密码' : '显示密码'"
    >
      <!-- 闭眼图标 - 密码隐藏时显示 -->
      <svg v-if="!showPassword" class="eye-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/>
        <line x1="1" y1="1" x2="23" y2="23"/>
      </svg>
      <!-- 睁眼图标 - 密码显示时显示 -->
      <svg v-else class="eye-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
        <circle cx="12" cy="12" r="3"/>
      </svg>
    </button>
  </div>
</template>

<style scoped>
.password-input-wrapper {
  position: relative;
  width: 100%;
}

.password-input {
  width: 100%;
  padding: 8px 36px 8px 12px;
  border: 1px solid #dadce0;
  border-radius: 4px;
  font-size: 13px;
  background: white;
  box-sizing: border-box;
}

.password-input:focus {
  outline: none;
  border-color: #0d9488;
}

/* 隐藏浏览器原生的密码显示按钮 */
.password-input::-webkit-credentials-auto-fill-button,
.password-input::-webkit-reveal {
  display: none !important;
  visibility: hidden !important;
  pointer-events: none !important;
}

/* Edge 浏览器 */
.password-input::-ms-reveal {
  display: none !important;
}

.toggle-btn {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  padding: 4px;
  cursor: pointer;
  color: #5f6368;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: background-color 0.2s, color 0.2s;
}

.toggle-btn:hover {
  background-color: #f1f3f4;
  color: #0d9488;
}

.eye-icon {
  width: 18px;
  height: 18px;
}
</style>
