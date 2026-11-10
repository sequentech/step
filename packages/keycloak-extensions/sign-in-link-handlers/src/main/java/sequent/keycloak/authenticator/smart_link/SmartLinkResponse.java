// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import com.fasterxml.jackson.annotation.JsonProperty;
import lombok.Data;

@Data
public class SmartLinkResponse {
  @JsonProperty("user_id")
  private String userId;

  @JsonProperty("link")
  private String link;

  @JsonProperty("sent")
  private boolean sent;

  /** Why the link was not sent, when it was asked for and not sent (R7-15); absent otherwise. */
  @JsonProperty("reason")
  @com.fasterxml.jackson.annotation.JsonInclude(
      com.fasterxml.jackson.annotation.JsonInclude.Include.NON_NULL)
  private String reason;
}
