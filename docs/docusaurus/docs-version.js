/*
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
*/

// The version of this documentation site, derived from BASE_URL. The
// Documentation workflow publishes `main` at /docusaurus/main and each
// release branch at /docusaurus/release/<X.Y>.
const SITE = 'https://docs.sequentech.io/docusaurus';

// The release lines in service. Add a line here when a release branch is cut.
const sites = [
  {label: 'Next (main)', url: `${SITE}/main/`},
  {label: '10.0', url: `${SITE}/release/10.0/`},
  {label: '9.0', url: `${SITE}/release/9.0/`},
];

function versionName(baseUrl) {
  const release = /\/release\/(\d+\.\d+)/.exec(baseUrl || '');
  if (release) return release[1];
  if (/\/main\/?$/.test(baseUrl || '')) return 'main';
  return 'dev';
}

const name = versionName(process.env.BASE_URL);

module.exports = {
  name,
  label: name === 'main' ? 'Next' : name === 'dev' ? 'Versions' : name,
  sites,
  versionName,
};
