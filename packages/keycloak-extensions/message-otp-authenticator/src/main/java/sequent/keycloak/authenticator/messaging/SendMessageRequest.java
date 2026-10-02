// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;

/** Body of harvest's {@code POST /messages/send}. */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record SendMessageRequest(
    @JsonProperty("tenant_id") String tenantId,
    @JsonProperty("election_event_id") String electionEventId,
    @JsonProperty("voter_id") String voterId,
    @JsonProperty("channel") MessageChannel channel,
    @JsonProperty("purpose") MessagePurpose purpose,
    @JsonProperty("destination") String destination,
    @JsonProperty("language") String language,
    @JsonProperty("content") MessageContent content,
    @JsonProperty("logical_key") String logicalKey,
    @JsonProperty("expires_at") String expiresAt) {

  @Override
  public String toString() {
    return "SendMessageRequest[channel=" + channel + ", purpose=" + purpose + "]";
  }
}
