import { defineConfig } from 'vitepress';
import { vitepress as layup } from '@hxyulin/layup/markdown-it';

export default defineConfig({
  title: 'layup in VitePress',
  markdown: {
    config: (md) => md.use(layup),
  },
});
