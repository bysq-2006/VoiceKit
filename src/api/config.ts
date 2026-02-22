import { invoke } from '@tauri-apps/api/core';

/**
 * ASR 提供商类型
 */
export type ASRProviderType = 'doubao' | 'xunfei';

/**
 * ASR 配置（与后端 AsrConfig 对应）
 */
export interface ASRConfig {
  provider: ASRProviderType;
  // 通用字段（与后端一致）
  api_id?: string;        // App ID / Access Key ID
  api_key?: string;       // Access Key / API Key
  api_secret?: string;    // API Secret（讯飞需要）
  // 豆包特有
  resource_id?: string;
}

/**
 * 应用配置结构（与后端 AppConfig 对应）
 */
export interface AppConfig {
  /** API 密钥（已迁移到 asr.access_key） */
  api_key: string | null;
  /** 全局快捷键，如 "Shift+E" */
  shortcut: string;
  /** 主题 */
  theme: 'system' | 'dark' | 'light';
  /** 开机自启 */
  auto_start: boolean;
  /** ASR 配置 */
  asr: ASRConfig;
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
  return await invoke('sync_config', { newConfig: config });
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

/**
 * 测试 ASR 配置
 */
export async function testASRConfig(asrConfig: ASRConfig): Promise<void> {
  return await invoke('test_asr_config', { config: asrConfig });
}
