/*
 * SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
 * SPDX-License-Identifier: AGPL-3.0-only
 */

import React from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import {useDocsVersion} from '@docusaurus/plugin-content-docs/client';

const labels = {
  en: 'Download this manual as PDF',
  es: 'Descargar este manual en PDF',
};

// Links to the PDF that scripts/manual-pdf.mjs prints after the build.
export default function ManualPdfLink() {
  const {i18n} = useDocusaurusContext();
  const {version} = useDocsVersion();
  const name = version === 'current' ? 'next' : version;
  const locale = i18n.currentLocale;
  const href = useBaseUrl(`/pdf/sequent-admin-manual-${name}-${locale}.pdf`);
  return (
    <p>
      <a className="button button--primary" href={href} download target="_blank" rel="noopener">
        {labels[locale] || labels.en}
      </a>
    </p>
  );
}
