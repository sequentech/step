/*
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
*/

// The manuals that `yarn pdf` prints and that <ManualPdfLink manual="..." />
// links to. Each manual is the sidebar section of its home page, relative to
// the base URL of the site.
//
// A guide prints only `pages` (paths below the home page),
// in that order, after the page `intro` of scripts/pdf-intros/<intro>.<locale>.html.
// The last page of a guide has no "Next" section. `link` replaces the label of
// the download button.
const {manualHome} = require('./docs-version.js');

const manuals = [
  {
    id: 'admin',
    file: 'sequent-admin-manual',
    home: manualHome,
    locales: ['en', 'es'],
    titles: {
      en: {
        title: 'Election Administrator Manual',
        productName: 'Sequent Online Voting, admin portal',
      },
      es: {
        title: 'Manual del administrador electoral',
        productName: 'Sequent Online Voting, portal de administración',
      },
    },
  },
  {
    id: 'ballot-audit',
    file: 'sequent-ballot-audit-guide',
    home: 'docs/election_auditors/ballot_audit_manual',
    pages: ['procedures/restore', 'procedures/run-the-audit'],
    intro: 'ballot-audit-guide',
    locales: ['en'],
    titles: {
      en: {
        title: 'Ballot Audit Guide',
        productName: 'Sequent Online Voting, ballot audit tools',
        link: 'Download the guide as PDF',
      },
    },
  },
];

module.exports = manuals;
