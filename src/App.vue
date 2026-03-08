<script setup lang="ts">
import { computed } from 'vue';
import { useConfig } from './composables/useConfig';
import { getThemeComponent } from './themes/index';

const { config } = useConfig();
const ThemeComponent = computed(() => getThemeComponent(config.value.theme));
</script>

<template>
  <div class="app-container">
    <Suspense>
      <component :is="ThemeComponent" :key="config.theme" />
      <template #fallback>
        <div class="loading">加载中...</div>
      </template>
    </Suspense>
  </div>
</template>

<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { background: transparent; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
#app { width: 100vw; height: 100vh; }
.app-container { width: 100%; height: 100%; }
.loading { display: flex; align-items: center; justify-content: center; width: 100%; height: 100%; color: #666; font-size: 14px; }
</style>
