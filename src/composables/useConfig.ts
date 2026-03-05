import { ref, onMounted, onUnmounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { ThemeName } from '../themes/index'

export type ASRProviderType = 'doubao' | 'xunfei'

export interface ASRConfig {
  provider: ASRProviderType
  doubao: { app_id: string; api_key: string }
  xunfei: { app_id: string; api_key: string; api_secret: string }
}

export interface AppConfig {
  shortcut: string
  auto_start: boolean
  theme: ThemeName
  asr: ASRConfig
}

const defaultConfig: AppConfig = {
  shortcut: '',
  auto_start: false,
  theme: 'default',
  asr: {
    provider: 'doubao',
    doubao: { app_id: '', api_key: '' },
    xunfei: { app_id: '', api_key: '', api_secret: '' },
  },
}

// 全局共享的配置状态
const globalConfig: Ref<AppConfig> = ref<AppConfig>({ ...defaultConfig })
const isLoaded = ref(false)

/**
 * 配置管理 composable
 * 所有实例共享同一个全局状态，当后端配置更新时自动同步
 */
export function useConfig() {
  const loading = ref(false)
  let unlistenConfig: UnlistenFn | undefined

  /** 从后端加载配置 */
  async function load() {
    loading.value = true
    try {
      globalConfig.value = await invoke<AppConfig>('get_config')
      isLoaded.value = true
    } finally {
      loading.value = false
    }
  }

  /** 保存到后端 */
  async function save() {
    await invoke('sync_config', { newConfig: globalConfig.value })
  }

  /** 更新单个配置项并自动保存 */
  async function update<K extends keyof AppConfig>(key: K, value: AppConfig[K]) {
    globalConfig.value[key] = value
    await save()
  }

  // 监听后端配置更新事件
  onMounted(async () => {
    unlistenConfig = await listen<AppConfig>('config-updated', (event) => {
      globalConfig.value = event.payload
    })
  })

  onUnmounted(() => {
    unlistenConfig?.()
  })

  // 如果全局状态还未加载，自动加载
  if (!isLoaded.value && typeof window !== 'undefined') {
    load()
  }

  return { config: globalConfig, loading, load, save, update }
}
