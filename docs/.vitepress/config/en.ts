import { defineConfig, type DefaultTheme } from 'vitepress'

export const en = defineConfig({
  lang: 'en-US',
  description: 'Make your app distribution with ease.',

  themeConfig: {
    nav: nav(),

    sidebar: {
      '/': { base: '/', items: sidebarGuide() },
    },

    editLink: {
      pattern:
        'https://github.com/fastforgedev/fastforge/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2021-present LiJianying',
    },
  },
})

function nav(): DefaultTheme.NavItem[] {
  return [
    {
      text: 'Docs',
      link: '/getting-started',
      activeMatch: '/getting-started',
    },
    {
      text: 'LeanFlutter',
      link: 'https://leanflutter.dev',
    },
  ]
}

function sidebarGuide(): DefaultTheme.SidebarItem[] {
  return [
    {
      text: 'Guide',
      collapsed: false,
      items: [
        { text: 'Getting Started', link: 'getting-started' },
        { text: 'Distribute Options', link: 'distribute-options' },
        { text: 'CLI', link: 'cli' },
      ],
    },
    {
      text: 'Makers',
      collapsed: false,
      items: [
        { text: 'aab', link: 'makers/aab' },
        { text: 'apk', link: 'makers/apk' },
        { text: 'app', link: 'makers/app' },
        { text: 'appimage', link: 'makers/appimage' },
        { text: 'deb', link: 'makers/deb' },
        { text: 'dmg', link: 'makers/dmg' },
        { text: 'exe', link: 'makers/exe' },
        { text: 'hap', link: 'makers/hap' },
        { text: 'ipa', link: 'makers/ipa' },
        { text: 'msix', link: 'makers/msix' },
        { text: 'pkg', link: 'makers/pkg' },
        { text: 'rpm', link: 'makers/rpm' },
        { text: 'zip', link: 'makers/zip' },
      ],
    },
    {
      text: 'Publishers',
      collapsed: false,
      items: [
        { text: 'App Gallery', link: 'publishers/appgallery' },
        { text: 'App Store', link: 'publishers/appstore' },
        { text: 'fir.im', link: 'publishers/fir' },
        {
          text: 'Firebase Hosting',
          link: 'publishers/firebase-hosting',
        },
        { text: 'Firebase', link: 'publishers/firebase' },
        { text: 'GitHub', link: 'publishers/github' },
        { text: 'Pgyer', link: 'publishers/pgyer' },
        { text: 'Play Store', link: 'publishers/playstore' },
        { text: 'Qiniu', link: 'publishers/qiniu' },
        { text: 'Vercel', link: 'publishers/vercel' },
      ],
    },
    {
      text: 'Tools',
      collapsed: false,
      items: [
        {
          text: 'Parse App Package',
          link: 'tools/parse-app-package',
        },
      ],
    },
  ]
}
