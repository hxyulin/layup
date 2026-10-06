import { defineConfig } from 'vitepress';
import { vitepress as layup } from '@hxyulin/layup/markdown-it';

const base = process.env.LAYUP_DOCS_BASE || '/layup/';

export default defineConfig({
  title: 'Layup',
  description: 'Diagrams for explaining software. Learn the DSL, edit examples live, and export SVG, HTML, or scene data.',
  base,
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: `${base}mark.svg` }],
    ['link', { rel: 'icon', type: 'image/svg+xml', href: `${base}mark-dark.svg`, media: '(prefers-color-scheme: dark)' }],
  ],
  cleanUrls: false,
  lastUpdated: true,
  markdown: { config: md => md.use(layup, { strict: true }) },
  vite: {
    worker: { format: 'es' },
    build: { assetsInlineLimit: 0 },
    // Keep the browser and build-time adapters on the same workspace engine.
    optimizeDeps: { exclude: ['@hxyulin/layup'] },
  },
  themeConfig: {
    logo: { light: '/mark.svg', dark: '/mark-dark.svg', alt: '' },
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Examples', link: '/examples' },
      { text: 'Playground', link: '/playground' },
      { text: 'Reference', link: '/reference/dsl' },
    ],
    sidebar: [
      { text: 'Learn Layup', items: [
        { text: 'Getting started', link: '/guide/getting-started' },
        { text: 'The source language', link: '/guide/language' },
        { text: 'Experimental language revision', link: '/guide/language-v1' },
        { text: 'Layout and routing', link: '/guide/layout' },
        { text: 'Styling and international text', link: '/guide/styling' },
      ] },
      { text: 'Diagram types', items: [
        { text: 'Architecture diagrams', link: '/diagrams/architecture' },
        { text: 'Decisions and flowcharts', link: '/diagrams/decisions' },
        { text: 'State machines', link: '/diagrams/states' },
        { text: 'Sequence diagrams', link: '/diagrams/sequences' },
      ] },
      { text: 'Present and reuse', items: [
        { text: 'Slides and progressive reveal', link: '/guide/presentations' },
        { text: 'Shared models and views', link: '/guide/models' },
        { text: 'Examples gallery', link: '/examples' },
        { text: 'Live playground', link: '/playground' },
      ] },
      { text: 'Use in your tools', items: [
        { text: 'Output formats', link: '/guide/formats' },
        { text: 'CLI, Rust and JavaScript', link: '/reference/api' },
        { text: 'Generate from analysis', link: '/guide/code-analysis' },
        { text: 'Markdown and VitePress', link: '/guide/markdown' },
        { text: 'Diagnostics and formatting', link: '/guide/tooling' },
        { text: 'DSL reference', link: '/reference/dsl' },
        { text: 'Develop and publish these docs', link: '/contributing' },
      ] },
    ],
    search: { provider: 'local', options: {
      _render(src, env, md) {
        // Search prose and source, not embedded font data or SVG stylesheets.
        return md.render(src, env).replace(/<svg\b[\s\S]*?<\/svg>/g, '');
      },
    } },
    socialLinks: [{ icon: 'github', link: 'https://github.com/hxyulin/layup' }],
    editLink: { pattern: 'https://github.com/hxyulin/layup/edit/main/docs/site/:path' },
    footer: { message: 'MIT OR Apache-2.0 · Bundled fonts: SIL OFL 1.1' },
  },
});
