// @ts-check

// Docusaurus builds one site per locale and sets this variable for each.
const locale = process.env.DOCUSAURUS_CURRENT_LOCALE || 'en';
const docsVersion = require('./docs-version');

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Sequent Online Voting',
  tagline: 'End-to-end verifiable and transparent online voting',
  url: 'https://your-docusaurus-site.example.com',
  baseUrl: process.env.BASE_URL || '/',
  favicon: 'img/favicon.ico',

  customFields: {
    docsVersion: docsVersion.name,
  },

  onBrokenLinks: 'throw',
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

  themes: ['@docusaurus/theme-mermaid'],
  markdown: {
    mermaid: true,
  },

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
