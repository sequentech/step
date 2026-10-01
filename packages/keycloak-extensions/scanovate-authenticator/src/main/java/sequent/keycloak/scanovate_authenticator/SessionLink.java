// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

/** One-time B-Trust flow URL together with the process id that identifies the session. */
public record SessionLink(String url, String processId) {}
