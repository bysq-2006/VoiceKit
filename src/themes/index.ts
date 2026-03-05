import { defineAsyncComponent, type Component } from 'vue';

export interface ThemeConfig {
  name: string;
  displayName: string;
  loader: () => Promise<Component>;
}

export const THEMES: ThemeConfig[] = [
  { name: 'default', displayName: '默认风格', loader: () => import('./default/index.vue') },
  { name: 'apple', displayName: 'Apple 风格', loader: () => import('./apple/index.vue') },
];

export type ThemeName = (typeof THEMES)[number]['name'];

export function getThemeComponent(theme: string) {
  const config = THEMES.find(t => t.name === theme) || THEMES[0];
  return defineAsyncComponent({
    loader: config.loader,
    timeout: 5000,
  });
}
