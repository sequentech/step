// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Bulletin Board Documentation',
  tagline: 'Sequent Bulletin Board Documentation. Sequent is a next-generation online voting platform purposely built to ensure the highest level of confidence in digital elections for election managers, voters and auditors.',
  favicon: 'img/favicon.png',

  // Set the production url of your site here
  url: 'https://sequentech-bulletin-board.netlify.app',

  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: '/',

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: 'sequentech', // Usually your GitHub org/user name.
  projectName: 'bulletin-board', // Usually your repo name.

  markdown: {
    mermaid: true,
  },

  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',

  // Even if you don't use internalization, you can use this field to set useful
  // metadata like html lang. For example, if your site is Chinese, you may want
  // to replace "en" with "zh-Hans".
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
          sidebarPath: require.resolve('./sidebars.js'),
          // Please change this to your repo.
          // Remove this to remove the "edit this page" links.
          editUrl:
            'https://github.com/sequentech/bulletin-board/tree/main/doc',
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      }),
    ],
  ],

  themes: ['@docusaurus/theme-mermaid'],
  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      colorMode: {
        defaultMode: 'light',
        disableSwitch: false,
        respectPrefersColorScheme: true,
      },
      mermaid: {
        theme: {light: 'default', dark: 'dark'},
      },
      navbar: {
        title: 'Bulletin Board',
        logo: {
          alt: 'Sequent',
          src: './img/sequent_docs_logo.png',
        },
        items: [
          {
            type: 'doc',
            docId: 'tutorials/get-started/get-started',
            position: 'left',
            label: 'Get Started',
          },
          {
            href: 'https://sequentech.io/',
            label: 'Website',
            position: 'right',
          },
          {
            href: 'https://github.com/sequentech/roadmap/discussions',
            label: 'Forums',
            position: 'right',
          },
          {
            label: 'Chat',
            href: 'https://discord.gg/WfvSTmcdY8',
            position: 'right',
          },
          {
            href: 'https://github.com/sequentech/',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [],
        copyright: `Copyright © ${new Date().getFullYear()} Sequent`,
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
        additionalLanguages: ['rust', 'toml'],
      },
    }),
};

module.exports = config;
