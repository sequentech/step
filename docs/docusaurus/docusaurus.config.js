/*
/* SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
*/



// @ts-check

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Sequent Online Voting',
  tagline: 'End-to-end verifiable and transparent online voting',
  url: 'https://sequentech.github.io',
  baseUrl: process.env.BASE_URL || '/step/',
  projectName: 'step',
  organizationName: 'sequentech',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,
  favicon: 'img/favicon.ico',

  onBrokenLinks: 'warn',
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
          editUrl:
            'https://github.com/sequentech/step/edit/main/docs/docusaurus',
          async sidebarItemsGenerator({
            defaultSidebarItemsGenerator,
            ...args
          }) {
            const sidebarItems = await defaultSidebarItemsGenerator(args);
  
            function removeDoc(items) {
              return items
                .filter((item) => {
                  if (item.type === 'doc' && item.id === 'rust_docs') {
                    return false;
                  }
                  return true;
                })
                .map((item) => {
                  if (item.type === 'category' && item.items) {
                    return {
                      ...item,
                      items: removeDoc(item.items),
                    };
                  }
                  return item;
                });
            }
  
            return removeDoc(sidebarItems);
          },
        },
        // completely remove the blog
        blog: false,
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      }),
    ],
  ],

  themes: ['@docusaurus/theme-mermaid'],
  markdown: {
    mermaid: true,
  },

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      prism: {
        additionalLanguages: ['php', 'bash', 'json', 'yaml', 'rust', 'java'],
      },
      navbar: {
        title: '',
        logo: {
          alt: 'Sequent Logo',
          src: '/img/logo_negative.svg',
          href: (process.env.BASE_URL || '') + '/docs/system_introduction',
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'docs',      // <-- matches the sidebar ID in sidebars.js
            position: 'left',
            label: 'Docs',
          },
          {
            href: (process.env.BASE_URL || '') + '/graphql',
            label: 'GraphQL API',
            position: 'left',
            target: '_blank',
          },
          {
            type: 'doc',
            docId: 'rust_docs',
            label: 'Rust Docs',
            position: 'left',
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
      mermaid: {
        theme: {light: 'neutral', dark: 'dark'},
      },
    }),
};

module.exports = config;
