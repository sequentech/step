// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import java.util.Arrays;
import java.util.Map;
import java.util.stream.Collectors;

/** What a Keycloak request does when its electoral-log event cannot be enqueued. */
enum PublishFailurePolicy {
  /** The event is enqueued before Keycloak commits; if that fails, the request fails. */
  FAIL_REQUEST("fail-request"),
  /** The event is enqueued after Keycloak commits; if that fails, it is logged and lost. */
  LOG_AND_CONTINUE("log-and-continue");

  static final String ENV = "ELECTORAL_LOG_PUBLISH_FAILURE_POLICY";

  private final String value;

  PublishFailurePolicy(String value) {
    this.value = value;
  }

  String value() {
    return value;
  }

  static PublishFailurePolicy fromEnvironment(Map<String, String> environment) {
    String configured = environment.get(ENV);
    if (configured == null || configured.isBlank()) {
      return FAIL_REQUEST;
    }
    return Arrays.stream(values())
        .filter(policy -> policy.value.equals(configured.trim()))
        .findFirst()
        .orElseThrow(
            () ->
                new IllegalStateException(
                    ENV
                        + " must be one of "
                        + Arrays.stream(values())
                            .map(PublishFailurePolicy::value)
                            .collect(Collectors.joining(", "))
                        + ", got "
                        + configured));
  }
}
