import { invoke } from '@tauri-apps/api/core';

/**
 * 应用配置结构（与后端 AppConfig 对应）
 */
export interface AppConfig {
  /** API 密钥 */
  api_key: string | null;
  /** 全局快捷键，如 "Shift+E" */
  shortcut: string;
  /** 主题 */
  theme: 'system' | 'dark' | 'light';
  /** 开机自启 */
  auto_start: boolean;
}

/**
 * 获取完整配置（从后端 state.config）
 */
export async function getConfig(): Promise<AppConfig> {
  return await invoke('get_config');
}

/**
 * 同步配置到后端（保存到 store 并应用）
 * @param config 完整的配置对象
 */
export async function syncConfig(config: AppConfig): Promise<void> {
  return await invoke('sync_config', { config });
}

/**
 * 更新单个配置项（先获取完整配置，修改后同步）
 */
export async function updateConfig<K extends keyof AppConfig>(
  key: K, 
  value: AppConfig[K]
): Promise<void> {
  const config = await getConfig();
  config[key] = value;
  return await syncConfig(config);
}
