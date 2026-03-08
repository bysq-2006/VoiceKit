import { defineAsyncComponent, type Component } from 'vue';

export interface ThemeSize {
  width: number;
  height: number;
}

export interface ThemeConfig {
  name: string;
  displayName: string;
  size: ThemeSize;
  loader: () => Promise<Component>;
}

export const THEMES: ThemeConfig[] = [
  { 
    name: 'default', 
    displayName: '默认风格', 
    size: { width: 160, height: 100 },
    loader: () => import('./default/index.vue') 
  },
  {
    name: 'google',
    displayName: '谷歌风格',
    size: { width: 420, height: 100 },
    loader: () => import('./google/index.vue')
  },
];

export type ThemeName = (typeof THEMES)[number]['name'];

export function getThemeComponent(theme: string) {
  const config = THEMES.find(t => t.name === theme) || THEMES[0];
  return defineAsyncComponent({
    loader: config.loader,
    timeout: 5000,
  });
}

export function getThemeSize(theme: string): ThemeSize {
  const config = THEMES.find(t => t.name === theme);
  return config?.size || THEMES[0].size;
}
