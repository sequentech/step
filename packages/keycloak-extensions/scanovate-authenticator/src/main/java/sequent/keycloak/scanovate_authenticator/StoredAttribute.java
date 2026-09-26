// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

/** A value extracted from the verification results and stored as an authentication note. */
public record StoredAttribute(String key, String value, String type) {}
