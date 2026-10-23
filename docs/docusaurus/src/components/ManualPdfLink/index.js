/*
 * SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
 * SPDX-License-Identifier: AGPL-3.0-only
 */

import React from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';

const labels = {
  en: 'Download this manual as PDF',
  es: 'Descargar este manual en PDF',
};

// Shown when the manual has no PDF in the language on screen.
const englishLabels = {
  es: 'Descargar este manual en PDF (inglés)',
};

// Links to the PDF of a manual of manuals.js (passed in the site config),
// which scripts/manual-pdf.mjs prints after the build.
export default function ManualPdfLink({manual = 'admin'}) {
  const {i18n, siteConfig} = useDocusaurusContext();
  const name = siteConfig.customFields.docsVersion;
  const entry = siteConfig.customFields.manuals.find((m) => m.id === manual);
  const locale = i18n.currentLocale;
  const pdfLocale = entry && entry.locales.includes(locale) ? locale : i18n.defaultLocale;
  const localeHref = useBaseUrl(`/pdf/${entry?.file}-${name}-${locale}.pdf`);
  // A PDF of the default locale is below the site's base URL, not the locale's.
  const defaultHref =
    siteConfig.baseUrl.replace(new RegExp(`${locale}/$`), '') +
    `pdf/${entry?.file}-${name}-${pdfLocale}.pdf`;
  // PR previews and local development builds have no PDF.
  if (name === 'dev' || !entry) return null;
  const href = pdfLocale === locale ? localeHref : defaultHref;
  const label =
    pdfLocale === locale
      ? labels[locale] || labels.en
      : englishLabels[locale] || labels.en;
  return (
    <p className="manual-pdf-link">
      <a className="button button--primary" href={href} download target="_blank" rel="noopener">
        {label}
      </a>
    </p>
  );
}
