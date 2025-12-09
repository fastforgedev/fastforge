import { defineConfig } from 'vitepress'
import {
  groupIconMdPlugin,
  groupIconVitePlugin,
  localIconLoader,
} from 'vitepress-plugin-group-icons'
import { search as zhSearch } from './zh'

const googleAnalyticsId = 'G-EC75P3JKR5'

export const shared = defineConfig({
  title: 'Fastforge',

  rewrites: {
    'en/:rest*': ':rest*',
  },

  lastUpdated: true,
  cleanUrls: true,
  metaChunk: true,

  markdown: {
    math: true,
    codeTransformers: [
      // We use `[!!code` in demo to prevent transformation, here we revert it back.
      {
        postprocess(code) {
          return code.replace(/\[\!\!code/g, '[!code')
        },
      },
    ],
    config(md) {
      // TODO: remove when https://github.com/vuejs/vitepress/issues/4431 is fixed
      const fence = md.renderer.rules.fence!
      md.renderer.rules.fence = function (
        tokens,
        idx,
        options,
        env,
        self
      ) {
        const { localeIndex = 'root' } = env
        const codeCopyButtonTitle = (() => {
          switch (localeIndex) {
            case 'zh':
              return '复制代码'
            default:
              return 'Copy code'
          }
        })()
        return fence(tokens, idx, options, env, self).replace(
          '<button title="Copy Code" class="copy"></button>',
          `<button title="${codeCopyButtonTitle}" class="copy"></button>`
        )
      }
      md.use(groupIconMdPlugin)
    },
  },

  sitemap: {
    hostname: 'https://fastforge.dev',
    transformItems(items) {
      return items.filter((item) => !item.url.includes('migration'))
    },
  },

  /* prettier-ignore */
  head: [
    ['link', { rel: 'icon', type: 'image/png', href: '/logo-mini.png' }],
    ['meta', { name: 'theme-color', content: '#155dfc' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:locale', content: 'en' }],
    ['meta', { property: 'og:title', content: 'Fastforge | Make your app distribution with ease.' }],
    ['meta', { property: 'og:site_name', content: 'Fastforge' }],
    ['meta', { property: 'og:url', content: 'https://fastforge.dev/' }],
    [
      'script',
      { async: '', src: `https://www.googletagmanager.com/gtag/js?id=${googleAnalyticsId}` }
    ],
    [
      'script',
      {},
      `window.dataLayer = window.dataLayer || [];
      function gtag(){dataLayer.push(arguments);}
      gtag('js', new Date());
      gtag('config', '${googleAnalyticsId}');`
    ],
    // [
    //   'script',
    //   {
    //     async: '',
    //     src: `https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js?client=ca-pub-6049036475236211`,
    //     crossorigin: 'anonymous',
    //   },
    // ],
  ],

  themeConfig: {
    logo: { src: '/logo-mini.png', width: 24, height: 24 },

    socialLinks: [
      {
        icon: 'github',
        link: 'https://github.com/fastforgedev/fastforge',
      },
    ],

    search: {
      provider: 'local',
      options: {
        locales: {
          ...zhSearch,
        },
      },
    },
  },
  vite: {
    plugins: [
      groupIconVitePlugin({
        customIcon: {
          fastforge: localIconLoader(
            import.meta.url,
            '../../public/logo-mini.png'
          ),
          firebase: 'logos:firebase',
        },
      }),
    ],
  },
})
