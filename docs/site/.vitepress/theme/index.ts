import DefaultTheme from 'vitepress/theme';
import '@hxyulin/layup/client';
import Playground from './components/Playground.vue';
import './custom.css';

export default {
  extends: DefaultTheme,
  enhanceApp({ app, router }) {
    app.component('Playground', Playground);
    router.onAfterRouteChange = () => {
      if (typeof window !== 'undefined') window.dispatchEvent(new Event('layup:navigate'));
    };
  },
};
