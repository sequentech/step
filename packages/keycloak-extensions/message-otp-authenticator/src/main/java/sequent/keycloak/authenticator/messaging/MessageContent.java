// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * What a message says. The code is only set for codes, so an approved authentication template can
 * carry it; it is never logged or stored.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
@JsonIgnoreProperties(ignoreUnknown = true)
public record MessageContent(
    @JsonProperty("subject") String subject,
    @JsonProperty("text") String text,
    @JsonProperty("html") String html,
    @JsonProperty("template_parameters") List<String> templateParameters,
    @JsonProperty("code") String code) {

  public MessageContent {
    templateParameters = templateParameters == null ? List.of() : List.copyOf(templateParameters);
  }

  public static MessageContent text(String text) {
    return new MessageContent(null, text, null, List.of(), null);
  }

  @Override
  public String toString() {
    return "MessageContent[redacted]";
  }
}
