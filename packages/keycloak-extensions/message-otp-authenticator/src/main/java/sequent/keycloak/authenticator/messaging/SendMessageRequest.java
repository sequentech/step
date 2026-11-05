// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * Body of harvest's {@code POST /messages/send}. The template key says which message this is, so
 * harvest picks the approved template bound to it: {@code otp}, {@code otl} or the message key of
 * the notice.
 */
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
    @JsonProperty("expires_at") String expiresAt,
    @JsonProperty("template_key") String templateKey,
    @JsonProperty("election_ids") List<String> electionIds,
    @JsonProperty("recipient") MessageRecipient recipient) {

  /** A request for the voter's own channel (no {@code recipient}: harvest's default). */
  public SendMessageRequest(
      String tenantId,
      String electionEventId,
      String voterId,
      MessageChannel channel,
      MessagePurpose purpose,
      String destination,
      String language,
      MessageContent content,
      String logicalKey,
      String expiresAt,
      String templateKey,
      List<String> electionIds) {
    this(
        tenantId,
        electionEventId,
        voterId,
        channel,
        purpose,
        destination,
        language,
        content,
        logicalKey,
        expiresAt,
        templateKey,
        electionIds,
        null);
  }

  /** A request without the voter's elections: codes, which are not restricted by Post. */
  public SendMessageRequest(
      String tenantId,
      String electionEventId,
      String voterId,
      MessageChannel channel,
      MessagePurpose purpose,
      String destination,
      String language,
      MessageContent content,
      String logicalKey,
      String expiresAt,
      String templateKey) {
    this(
        tenantId,
        electionEventId,
        voterId,
        channel,
        purpose,
        destination,
        language,
        content,
        logicalKey,
        expiresAt,
        templateKey,
        null,
        null);
  }

  @Override
  public String toString() {
    return "SendMessageRequest[channel=" + channel + ", purpose=" + purpose + "]";
  }
}
