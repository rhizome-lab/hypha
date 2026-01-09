import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(
  defineConfig({
    title: 'Hypha',
    description: 'Federation protocol for persistent worlds',

    base: '/hypha/',

    themeConfig: {
      nav: [
        { text: 'Guide', link: '/introduction' },
        { text: 'Protocol', link: '/protocol' },
        { text: 'Security', link: '/security' },
        { text: 'Rhizome', link: 'https://rhizome-lab.github.io/' },
      ],

      sidebar: [
        {
          text: 'Guide',
          items: [
            { text: 'Introduction', link: '/introduction' },
            { text: 'Architecture', link: '/architecture' },
          ]
        },
        {
          text: 'Reference',
          items: [
            { text: 'Protocol', link: '/protocol' },
            { text: 'Security', link: '/security' },
            { text: 'Import Policies', link: '/import-policies' },
          ]
        },
      ],

      socialLinks: [
        { icon: 'github', link: 'https://github.com/rhizome-lab/hypha' }
      ],

      search: {
        provider: 'local'
      },

      editLink: {
        pattern: 'https://github.com/rhizome-lab/hypha/edit/master/docs/:path',
        text: 'Edit this page on GitHub'
      },
    },

    vite: {
      optimizeDeps: {
        include: ['mermaid'],
      },
    },
  }),
)
