/*
 * SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
 * SPDX-License-Identifier: AGPL-3.0-only
 */

import React from 'react';
import {useLocation} from '@docusaurus/router';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import {translate} from '@docusaurus/Translate';
import DropdownNavbarItem from '@theme/NavbarItem/DropdownNavbarItem';

function versionLabel(name) {
  if (name === 'main') {
    return translate({id: 'versionSites.next', message: 'Next'});
  }
  if (name === 'dev') {
    return translate({id: 'versionSites.versions', message: 'Versions'});
  }
  return name;
}

// Links the sites that each branch publishes (docs-version.js). The link
// keeps the language; from a page of the administrator manual it opens the
// manual of the other version, because a page can be missing there.
export default function VersionSitesNavbarItem({mobile, ...props}) {
  const {siteConfig, i18n} = useDocusaurusContext();
  const {pathname} = useLocation();
  const {docsVersion, docsSites, manualHome} = siteConfig.customFields;
  const localePrefix =
    i18n.currentLocale === i18n.defaultLocale ? '' : `${i18n.currentLocale}/`;
  const section = manualHome.slice(0, manualHome.lastIndexOf('/'));
  const inManual = pathname.includes(`/${section}`);
  const items = docsSites.map(({name, url, manualHome: home}) => ({
    label:
      name === 'main'
        ? translate({id: 'versionSites.main', message: 'Next (main)'})
        : name,
    href: `${url}${localePrefix}${inManual ? home : ''}`,
    className: name === docsVersion ? 'dropdown__link--active' : undefined,
  }));
  return (
    <DropdownNavbarItem
      {...props}
      mobile={mobile}
      label={versionLabel(docsVersion)}
      items={items}
    />
  );
}
