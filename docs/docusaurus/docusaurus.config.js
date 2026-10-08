/*
/* SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
*/



// @ts-check

// Docusaurus builds one site per locale and sets this variable for each.
const locale = process.env.DOCUSAURUS_CURRENT_LOCALE || 'en';
const docsVersion = require('./docs-version');

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

  customFields: {
    docsVersion: docsVersion.name,
  },

  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',

  // The site language selector. Untranslated pages fall back to the English source.
  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'es'],
    localeConfigs: {
      en: {label: 'English', htmlLang: 'en'},
      es: {label: 'Español', htmlLang: 'es'},
    },
  },
  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          path: 'docs',
          remarkPlugins: [require('./plugins/remark-code-tabs')],
          sidebarPath: require.resolve('./sidebars.js'),
          editUrl:
            'https://github.com/sequentech/step/edit/main/docs/docusaurus',
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
          href: '/docs/system_introduction',
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
            // Each release branch publishes its own site; this menu links them.
            type: 'dropdown',
            label: docsVersion.label,
            position: 'right',
            items: docsVersion.sites.map(({label, url}) => ({label, href: url})),
          },
          {
            type: 'localeDropdown',
            position: 'right',
          },
          {
            href: 'https://github.com/sequentech',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      ...(locale === 'es' && {
        announcementBar: {
          id: 'es-translation-in-progress',
          content:
            'La traducción al español está en curso. Las páginas sin traducir se muestran en inglés.',
          isCloseable: true,
        },
      }),
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
