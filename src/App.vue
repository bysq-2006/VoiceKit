<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';


// 麦克风按钮激活状态
const isActive = ref(false);

const toggleMic = () => {
  isActive.value = !isActive.value;
};

// 显示/隐藏窗口
const showWindow = async () => {
  await invoke('show_window');
};

const hideWindow = async () => {
  await invoke('hide_window');
};
</script>

<template>
  <div class="voice-panel">
    <!-- 自定义标题栏 -->
    <div class="titlebar" data-tauri-drag-region>
      <!-- 左边留空，用于平衡布局让拖动手柄居中 -->
      <div class="spacer"></div>

      <!-- 中间拖动手柄 -->
      <div class="drag-handle"></div>

      <!-- 右边关闭按钮（谷歌风格） -->
      <button class="close-btn" @click="hideWindow" title="隐藏窗口">×</button>
    </div>

    <!-- 中间按钮区域 -->
    <div class="controls">
      <!-- 左侧设置按钮 -->
      <button class="icon-btn" title="隐藏窗口（后台运行）">
        <svg viewBox="0 0 1024 1024">
          <path
            d="M880.48 451.488l-14.72-64 54.96-94.976-117.6-146.464-105.28 32L637.92 149.6 597.472 48H409.248l-40.464 101.6-59.904 28.544-105.28-32.064-117.616 146.448 54.96 94.976-14.72 64-90.976 61.872 42.016 182.864 108.944 16.512 41.312 51.456-8.016 109.216 169.696 80.976L470.064 880h66.592l80.864 74.4L787.2 873.328l-8-109.168 41.296-51.424 108.96-16.496 42-182.864-90.992-61.888z m-16.816 173.808l-86 13.056-80.576 100.32 6.336 86.16-71.664 34.08L567.808 800h-128.896l-63.968 58.912-71.664-34.176 6.336-86.112-80.576-100.304-86-13.04-17.568-76.528 71.696-48.768 28.8-125.28-43.296-74.832 49.424-61.552 83.248 25.328 116.224-55.408L463.52 128h79.664l31.952 80.24 116.224 55.488 83.248-25.36 49.424 61.536-43.296 74.816 28.8 125.28 71.712 48.752-17.6 76.544zM504.88 336c-92.64 0-167.968 75.36-167.968 168s75.36 168 168 168 168-75.36 168-168S597.536 336 504.88 336z m0.032 256a88.112 88.112 0 0 1-88-88c0-23.504 9.152-45.6 25.76-62.224A87.392 87.392 0 0 1 504.928 416c48.512 0 88 39.488 88 88s-39.488 88-88 88z"
            fill="#808080" />
        </svg>
      </button>

      <!-- 中间大按钮（麦克风） -->
      <button class="main-btn" :class="{ active: isActive }" @click="toggleMic">
        <!-- 遮罩层：从中心扩散，颜色反转 -->
        <span class="btn-mask"></span>
        <!-- 波纹环 -->
        <span class="btn-wave wave-1"></span>
        <span class="btn-wave wave-2"></span>
        <!-- 图标 -->
        <svg viewBox="0 0 1024 1024">
          <path
            d="M512 683.52c130.56 0 235.52-102.4 235.52-232.96V256c0-130.56-104.96-232.96-235.52-232.96s-235.52 102.4-235.52 232.96v194.56c0 130.56 102.4 232.96 235.52 232.96z m368.64-281.6c0-23.04-20.48-43.52-46.08-43.52s-43.52 20.48-43.52 43.52c0 5.12 0 10.24 2.56 12.8v33.28c0 151.04-125.44 276.48-281.6 276.48-153.6 0-281.6-125.44-281.6-276.48V409.6c0-2.56 2.56-5.12 2.56-10.24 0-23.04-20.48-43.52-43.52-43.52-25.6 0-43.52 20.48-43.52 43.52v64c0 186.88 140.8 335.36 320 360.96v87.04h-122.88c-25.6 0-46.08 20.48-46.08 46.08s20.48 43.52 46.08 43.52h332.8c28.16 0 43.52-17.92 43.52-43.52 0-23.04-17.92-46.08-43.52-46.08h-122.88v-87.04c184.32-20.48 327.68-174.08 327.68-360.96v-61.44z m0 0"
            fill="currentColor" />
        </svg>
      </button>

      <!-- 右侧帮助按钮 -->
      <button class="icon-btn" title="显示窗口">
        <svg viewBox="0 0 1024 1024">
          <path d="M512 32C247.04 32 32 247.04 32 512s215.04 480 480 480 480-215.04 480-480S776.96 32 512 32z m0 853.12c-206.08 0-373.12-167.68-373.12-373.12S305.92 138.88 512 138.88a373.12 373.12 0 1 1 0 746.24z" fill="#808080" />
          <path d="M518.4 260.48c-49.28 0-88.96 14.72-118.4 42.88-30.08 28.8-44.8 68.48-44.8 117.76 0 1.92 0 21.76 14.08 37.12 6.4 7.04 18.56 14.72 38.4 14.72 38.4 0 54.4-30.08 55.68-51.2 0-21.12 3.84-37.12 11.52-47.36 4.48-6.4 13.44-15.36 39.68-15.36 15.36 0 26.24 3.84 33.28 10.24 7.68 7.68 11.52 19.2 11.52 32.64 0 10.24-3.84 20.48-11.52 29.44l-5.12 6.4c-42.24 38.4-65.92 64.64-74.88 82.56-9.6 18.56-14.08 40.32-14.08 67.84v8.32c0 17.28 14.08 42.88 53.12 42.88 40.32 0 53.76-26.88 55.68-42.88v-8.32c0-10.88 1.92-20.48 7.04-30.08 4.48-8.96 10.88-16.64 19.2-23.68 26.88-23.68 46.08-40.96 54.4-50.56 17.28-23.04 26.24-51.84 26.24-85.76 0-42.24-14.08-76.16-42.24-101.12-28.16-23.68-64-36.48-108.8-36.48z" fill="#808080" />
          <path d="M508.8 711.68m-55.68 0a55.68 55.68 0 1 0 111.36 0 55.68 55.68 0 1 0-111.36 0Z" fill="#808080" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.voice-panel {
  width: 100%;
  height: 100%;
  background: linear-gradient(180deg, #f5f5f5 0%, #e8e8e8 100%);
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.3);
}

/* 自定义标题栏 */
.titlebar {
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 6px;
  background: rgba(0, 0, 0, 0.03);
  -webkit-app-region: drag;
  app-region: drag;
}

/* 左边占位，让拖动手柄能居中 */
.spacer {
  width: 20px;
  height: 20px;
}

/* 拖动手柄 */
.drag-handle {
  width: 30px;
  height: 3.5px;
  background: #959595;
  border-radius: 2px;
}

/* 关闭按钮 - 谷歌风格 */
.close-btn {
  width: 20px;
  height: 20px;
  border: none;
  background: transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  color: #5f6368;
  border-radius: 50%;
  transition: all 0.2s;
  font-weight: 300;
}

.close-btn:hover {
  background: rgba(95, 99, 104, 0.1);
  color: #202124;
}

/* 中间控制区域 */
.controls {
  flex: 1;
  display: flex;
  justify-content: space-evenly;
  align-items: center;
  gap: 12px;
  padding: 4px 12px 10px;
  -webkit-app-region: no-drag;
  transform: translateY(3px);
}

/* 主按钮（大圆按钮） */
.main-btn {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  border: 1.5px solid #0d9488;
  background: white;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.1);
  position: relative;
}

.main-btn:hover {
  transform: scale(1.05);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

/* 遮罩层：从中心扩散，颜色反转 */
.btn-mask {
  position: absolute;
  width: 100%;
  height: 100%;
  background: #0d9488;
  border-radius: 50%;
  transform: scale(0);
  transition: transform 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  pointer-events: none;
}

.main-btn.active .btn-mask {
  transform: scale(1);
}

/* 波纹环 - 网易云音乐风格，在按钮外围扩散 */
.btn-wave {
  position: absolute;
  width: 100%;
  height: 100%;
  border-radius: 50%;
  border: 4px solid rgba(13, 148, 136, 0.5);
  opacity: 0;
  pointer-events: none;
}

.main-btn.active .wave-1 {
  animation: sound-wave 2s ease-out infinite;
}

.main-btn.active .wave-2 {
  animation: sound-wave 2s ease-out infinite 0.6s;
}

@keyframes sound-wave {
  0% {
    transform: scale(1.1);
    opacity: 0.5;
  }

  100% {
    transform: scale(1.6);
    opacity: 0;
  }
}

/* 图标：在遮罩之上 */
.main-btn svg {
  position: relative;
  z-index: 1;
  width: 22px;
  height: 22px;
  color: #0d9488;
  transition: color 0.3s;
}

.main-btn.active svg {
  color: white;
}

/* 侧边小按钮 */
.icon-btn {
  width: 18px;
  border: none;
  background: transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  padding: 0;
}

.icon-btn:hover {
  opacity: 0.7;
}

.icon-btn svg {
  color: #666;
}
</style>

<style>
/* 全局样式 */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background: transparent;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

#app {
  width: 100vw;
  height: 100vh;
}
</style>
