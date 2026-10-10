// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Body of harvest's {@code POST /messages/link}. The authentication session and the challenge are
 * opaque digests; harvest holds the code encrypted until it is used, replaced or expires. For a
 * saved voter (sign-in, reset), {@code expected_recipient} is their recorded Page-scoped ID: the
 * code goes only to that account, so tapping Get Started on someone else's link tells nothing.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record CreateMessengerLinkRequest(
    @JsonProperty("tenant_id") String tenantId,
    @JsonProperty("election_event_id") String electionEventId,
    @JsonProperty("auth_session") String authSession,
    @JsonProperty("challenge") String challenge,
    @JsonProperty("code") String code,
    @JsonProperty("language") String language,
    @JsonProperty("content") MessageContent content,
    @JsonProperty("expires_at") String expiresAt,
    @JsonProperty("expected_recipient") String expectedRecipient) {

  /**
   * A link anyone may complete: while enrolling or changing the contact, the voter's account isn't
   * known yet.
   */
  public CreateMessengerLinkRequest(
      String tenantId,
      String electionEventId,
      String authSession,
      String challenge,
      String code,
      String language,
      MessageContent content,
      String expiresAt) {
    this(
        tenantId,
        electionEventId,
        authSession,
        challenge,
        code,
        language,
        content,
        expiresAt,
        null);
  }

  @Override
  public String toString() {
    return "CreateMessengerLinkRequest[redacted]";
  }
}
