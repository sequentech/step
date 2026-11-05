// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;

/** Progress of a Messenger link. The Page-scoped ID is only present once confirmed. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record MessengerLinkStatus(
    @JsonProperty("state") String state,
    @JsonProperty("page_scoped_id") String pageScopedId,
    @JsonProperty("page_id") String pageId) {

  @JsonIgnore
  public MessengerLinkState linkState() {
    return MessengerLinkState.parse(state);
  }

  @Override
  public String toString() {
    return "MessengerLinkStatus[state=" + state + "]";
  }
}
