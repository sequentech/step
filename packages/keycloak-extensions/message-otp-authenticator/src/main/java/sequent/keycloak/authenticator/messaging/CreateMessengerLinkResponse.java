// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;

/** The one-time reference, its m.me link and the word the voter can send instead. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record CreateMessengerLinkResponse(
    @JsonProperty("reference") String reference,
    @JsonProperty("link") String link,
    @JsonProperty("link_word") String linkWord,
    @JsonProperty("expires_at") String expiresAt) {

  @Override
  public String toString() {
    return "CreateMessengerLinkResponse[redacted]";
  }
}
