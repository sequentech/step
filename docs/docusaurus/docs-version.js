/*
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
*/

// The version of this documentation site, derived from BASE_URL. The
// Documentation workflow publishes `main` at /docusaurus/main and each
// release branch at /docusaurus/release/<name>.
const SITE = 'https://docs.sequentech.io/docusaurus';

// The release lines in service. Add a line here when a release branch is cut.
const sites = [
  {name: 'main', url: `${SITE}/main/`},
  {name: '10.1', url: `${SITE}/release/10.1/`},
  {name: '10.0', url: `${SITE}/release/10.0/`},
  {name: '9.0', url: `${SITE}/release/9.0/`},
];

// The home page of the election administrator manual, relative to the base
// URL. The PDF starts there and the version menu links to it.
const manualHome = 'docs/election_managers/election_management';

// "main", the full release branch name after "release/" (for example "10.0"
// or "9.0.0"), or "dev" for PR previews and local builds.
function versionName(baseUrl) {
  const release = /\/release\/([^/]+)\/?$/.exec(baseUrl || '');
  if (release) return release[1];
  if (/\/main\/?$/.test(baseUrl || '')) return 'main';
  return 'dev';
}

function versionLabel(name) {
  if (name === 'main') return 'Next';
  if (name === 'dev') return 'Versions';
  return name;
}

const name = versionName(process.env.BASE_URL);

module.exports = {
  name,
  label: versionLabel(name),
  sites,
  manualHome,
  versionName,
  versionLabel,
};
