// @ts-check

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Sequent Online Voting',
  tagline: 'End-to-end verifiable and transparent online voting',
  url: 'https://your-docusaurus-site.example.com',
  baseUrl: '/',
  favicon: 'img/favicon.ico',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  // i18n, if you ever need it:
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },
  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          path: 'docs',
          sidebarPath: require.resolve('./sidebars.js'),
          // remove editUrl if you don't want "edit this page" links
          editUrl:
            'https://github.com/sequentech/step/docs/docusaurus',
        },
        // completely remove the blog
        blog: false,
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      navbar: {
        title: '    Home',
        logo: {
          alt: 'Sequent Logo',
          src: '/img/logo_negative.svg',
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'docs',      // <-- matches the sidebar ID in sidebars.js
            position: 'left',
            label: 'Docs',
          },
          {
            href: 'https://github.com/sequentech',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        copyright: `Copyright © ${new Date().getFullYear()} Sequent`,
      },
      scripts: [
        '/js/custom-home-highlight.js',
      ],
    }),
};

module.exports = config;
