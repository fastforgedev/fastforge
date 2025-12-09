import { defineConfig, type DefaultTheme } from 'vitepress'

export const zh = defineConfig({
  lang: 'zh-Hans',
  description: 'Make your app distribution with ease.',

  themeConfig: {
    nav: nav(),

    sidebar: {
      '/zh/': { base: '/zh/', items: sidebarGuide() },
    },

    editLink: {
      pattern:
        'https://github.com/fastforgedev/fastforge/edit/main/docs/:path',
      text: '在 GitHub 上编辑此页面',
    },

    footer: {
      message: '基于 MIT 许可发布',
      copyright: `版权所有 © 2021-${new Date().getFullYear()} LiJianying`,
    },

    docFooter: {
      prev: '上一页',
      next: '下一页',
    },

    outline: {
      label: '页面导航',
    },

    lastUpdated: {
      text: '最后更新于',
      formatOptions: {
        dateStyle: 'short',
        timeStyle: 'medium',
      },
    },

    langMenuLabel: '多语言',
    returnToTopLabel: '回到顶部',
    sidebarMenuLabel: '菜单',
    darkModeSwitchLabel: '主题',
    lightModeSwitchTitle: '切换到浅色模式',
    darkModeSwitchTitle: '切换到深色模式',
    skipToContentLabel: '跳转到内容',
  },
})

function nav(): DefaultTheme.NavItem[] {
  return [
    {
      text: '文档',
      link: '/zh/getting-started',
      activeMatch: '/zh/getting-started',
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
      text: '指南',
      collapsed: false,
      items: [
        { text: '快速开始', link: 'getting-started' },
        { text: '分发选项', link: 'distribute-options' },
        { text: 'CLI', link: 'cli' },
      ],
    },
    {
      text: '制作器',
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
      text: '发布器',
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
      text: '工具',
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

export const search: DefaultTheme.AlgoliaSearchOptions['locales'] = {
  zh: {
    placeholder: '搜索文档',
    translations: {
      button: {
        buttonText: '搜索文档',
        buttonAriaLabel: '搜索文档',
      },
      modal: {
        searchBox: {
          resetButtonTitle: '清除查询条件',
          resetButtonAriaLabel: '清除查询条件',
          cancelButtonText: '取消',
          cancelButtonAriaLabel: '取消',
        },
        startScreen: {
          recentSearchesTitle: '搜索历史',
          noRecentSearchesText: '没有搜索历史',
          saveRecentSearchButtonTitle: '保存至搜索历史',
          removeRecentSearchButtonTitle: '从搜索历史中移除',
          favoriteSearchesTitle: '收藏',
          removeFavoriteSearchButtonTitle: '从收藏中移除',
        },
        errorScreen: {
          titleText: '无法获取结果',
          helpText: '你可能需要检查你的网络连接',
        },
        footer: {
          selectText: '选择',
          navigateText: '切换',
          closeText: '关闭',
          searchByText: '搜索提供者',
        },
        noResultsScreen: {
          noResultsText: '无法找到相关结果',
          suggestedQueryText: '你可以尝试查询',
          reportMissingResultsText: '你认为该查询应该有结果？',
          reportMissingResultsLinkText: '点击反馈',
        },
      },
    },
  },
}
