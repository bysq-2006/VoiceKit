<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

const isRecording = ref(false);
let unlistenState: UnlistenFn;
let hideTimeout: number | null = null;

onMounted(async () => {
  isRecording.value = await invoke('get_recording_state');
  unlistenState = await listen<boolean>('recording-state-changed', (e) => {
    const wasRecording = isRecording.value;
    isRecording.value = e.payload;
    
    // 如果停止录音，延迟 1.5 秒后隐藏窗口（等待遮罩动画完成）
    if (wasRecording && !isRecording.value) {
      hideTimeout = window.setTimeout(() => {
        invoke('hide_window');
      }, 600);
    } else if (isRecording.value && hideTimeout) {
      // 如果重新开始录音，取消隐藏
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }
  });
});

onUnmounted(() => {
  unlistenState?.();
  if (hideTimeout) {
    clearTimeout(hideTimeout);
  }
});

const toggleRecording = () => invoke('set_recording', { recording: !isRecording.value });
</script>

<template>
  <div class="panel" :class="{ recording: isRecording, active: isRecording }" data-tauri-drag-region>
    <div class="content">
      <div class="text">
        <span class="status">{{ isRecording ? 'Listening...' : 'Ready' }}</span>
        <div class="wave">
          <div v-for="i in 4" :key="i" class="bar" :class="{ active: isRecording }" :style="{ animationDelay: `${i * 0.1}s` }" />
        </div>
      </div>

      <button class="mic-btn" :class="{ active: isRecording }" @click.stop="toggleRecording">
        <svg viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z"/>
          <path d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"/>
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.panel {
  position: relative;
  width: 100%;
  height: 100%;
  background: transparent;
  border-radius: 32px;
  box-shadow:
    0 2px 8px rgba(0, 0, 0, 0.08),
    0 4px 20px rgba(0, 0, 0, 0.05);
  overflow: hidden;
}

.content {
  width: 100%;
  height: 100%;
  background: #f5f5f5;
  /* 圆心位置：X轴(左->右) Y轴(上->下) */
  /* 示例值：50% 50% = 中心，100% 50% = 右侧中间，50% 100% = 底部中间 */
  clip-path: circle(0% at 87% 50%);
  transition: clip-path 0.6s cubic-bezier(0.22, 1, 0.36, 1);
}

.panel.active .content {
  clip-path: circle(150% at 86% 50%);
}

:global(body) {
  overflow: hidden;
}

.content {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px 0 24px;
  -webkit-app-region: drag;
}

.text {
  transform: translateY(-4px);
  display: flex;
  flex-direction: column;
}

.status {
  font-size: 18px;
  font-weight: 500;
  color: #1a1a2e;
}

.wave {
  display: flex;
  align-items: flex-end;
  gap: 3px;
  height: 16px;
}

.bar {
  width: 4px;
  height: 4px;
  background: #9e9e9e;
  border-radius: 2px;
  transition: height 0.2s;
}

.bar.active {
  background: #4CAF50;
  animation: wave 0.8s ease-in-out infinite;
}

@keyframes wave {
  0%, 100% { height: 4px; }
  50% { height: 14px; }
}

.mic-btn {
  width:  50px;
  height: 50px;
  -webkit-app-region: no-drag;
  border: none;
  border-radius: 50%;
  background: white;
  color: #4CAF50;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s;
  box-shadow:
    0 0 0 4px rgba(0, 0, 0, 0.05),
    0 4px 12px rgba(0, 0, 0, 0.1);
}

.mic-btn:hover {
  transform: scale(1.05);
  box-shadow:
    0 0 0 6px rgba(0, 0, 0, 0.08),
    0 6px 16px rgba(0, 0, 0, 0.15);
}

.mic-btn.active {
  background: #4CAF50;
  color: white;
  box-shadow:
    0 0 0 4px rgba(76, 175, 80, 0.15),
    0 4px 12px rgba(76, 175, 80, 0.3);
  animation: glow 1.5s ease-in-out infinite;
}

.mic-btn.active:hover {
  box-shadow:
    0 0 0 6px rgba(76, 175, 50, 0.2),
    0 6px 16px rgba(76, 175, 80, 0.4);
}

.mic-btn svg {
  width: 28px;
  height: 28px;
}

@keyframes glow {
  0%, 100% {
    box-shadow:
      0 0 0 4px rgba(76, 175, 80, 0.15),
      0 0 20px rgba(76, 175, 80, 0.4);
  }
  50% {
    box-shadow:
      0 0 0 6px rgba(76, 175, 80, 0.25),
      0 0 30px rgba(76, 175, 80, 0.6);
  }
}
</style>
