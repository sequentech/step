// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.annotation.JsonProperty;

/** Body of harvest's {@code POST /messages/link/status} and {@code /messages/link/confirm}. */
public record MessengerLinkRequest(
    @JsonProperty("tenant_id") String tenantId,
    @JsonProperty("reference") String reference,
    @JsonProperty("auth_session") String authSession,
    @JsonProperty("challenge") String challenge) {

  @Override
  public String toString() {
    return "MessengerLinkRequest[redacted]";
  }
}
