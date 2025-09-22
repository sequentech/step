// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.credential;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;

public class MessageOTPSecretData {

  private final String nothing;

  @JsonCreator
  public MessageOTPSecretData(@JsonProperty("nothing") String nothing) {
    this.nothing = nothing;
  }

  public String getNothing() {
    return this.nothing;
  }
}
