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

// Links to the PDF that scripts/manual-pdf.mjs prints after the build.
export default function ManualPdfLink() {
  const {i18n, siteConfig} = useDocusaurusContext();
  const name = siteConfig.customFields.docsVersion;
  const locale = i18n.currentLocale;
  const href = useBaseUrl(`/pdf/sequent-admin-manual-${name}-${locale}.pdf`);
  // PR previews and local development builds have no PDF.
  if (name === 'dev') return null;
  return (
    <p className="manual-pdf-link">
      <a className="button button--primary" href={href} download target="_blank" rel="noopener">
        {labels[locale] || labels.en}
      </a>
    </p>
  );
}
