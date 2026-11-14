// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Login form values for one synthetic voter, shared by the k6 and Chromium engines.
// Mirrors packages/step-cli/src/load/census.rs: voters are grouped consecutively by
// absolute suffix, every group shares its match-attribute values, and each position
// in a group has its own password so colliding voters stay distinguishable.

const DATE_OF_BIRTH = "dateOfBirth";
const DAY_MS = 86400000;
const ORIGIN_MS = Date.UTC(1900, 0, 1);

/** Value shared by one collision group; dates count days from 1900-01-01. */
export function attributeValue(attribute, group) {
  if (attribute === DATE_OF_BIRTH)
    return new Date(ORIGIN_MS + group * DAY_MS).toISOString().slice(0, 10);
  return "g" + group;
}

/** Fields posted to the Keycloak login form for the voter with this absolute suffix. */
export function voterCredentials(config, index, password) {
  const login = config.login || {};
  const attributes = login.match_attributes || [];
  const depth = login.voters_per_value || 1;
  const group = Math.floor(index / depth);
  const credentials = {
    ...config.login_fields,
    username: config.username_prefix + index,
    password: depth === 1 ? password : password + "-" + (index % depth),
  };
  for (const attribute of attributes)
    credentials[attribute] = attributeValue(attribute, group);
  return credentials;
}
