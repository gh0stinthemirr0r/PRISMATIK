import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  kit: {
    adapter: adapter({
      fallback: 'index.html',
      precompress: false
    }),
    prerender: {
      entries: ['*', '/workspace'],
      // Dynamic instrument routes are resolved by the desktop SPA fallback;
      // there is no finite fixture list to crawl at build time.
      handleUnseenRoutes: 'ignore'
    }
  }
};

export default config;
