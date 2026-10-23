/*
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
*/

// The manuals that `yarn pdf` prints and that <ManualPdfLink manual="..." />
// links to. Each manual is the sidebar section of its home page, relative to
// the base URL of the site.
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
    file: 'sequent-ballot-audit-manual',
    home: 'docs/election_auditors/ballot_audit_manual',
    locales: ['en'],
    titles: {
      en: {
        title: 'Ballot Audit Manual',
        productName: 'Sequent Online Voting, ballot audit tools',
      },
    },
  },
];

module.exports = manuals;
