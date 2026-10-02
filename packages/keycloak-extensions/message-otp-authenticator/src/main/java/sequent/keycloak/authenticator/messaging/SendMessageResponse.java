// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;

/** Outcome of a send. The reason is short and never contains message content. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record SendMessageResponse(
    @JsonProperty("message_id") String messageId,
    @JsonProperty("state") String state,
    @JsonProperty("reason") String reason) {

  public static SendMessageResponse of(MessageAttemptState state, String reason) {
    return new SendMessageResponse(null, state.name(), reason);
  }

  @JsonIgnore
  public MessageAttemptState attemptState() {
    return MessageAttemptState.parse(state);
  }
}
