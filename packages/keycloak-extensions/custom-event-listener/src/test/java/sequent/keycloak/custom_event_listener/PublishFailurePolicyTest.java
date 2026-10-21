// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;

import java.util.Map;
import org.junit.jupiter.api.Test;

class PublishFailurePolicyTest {
  @Test
  void failRequestIsTheDefault() {
    assertEquals(PublishFailurePolicy.FAIL_REQUEST, PublishFailurePolicy.fromEnvironment(Map.of()));
    assertEquals(
        PublishFailurePolicy.FAIL_REQUEST,
        PublishFailurePolicy.fromEnvironment(Map.of(PublishFailurePolicy.ENV, " ")));
  }

  @Test
  void configuredPoliciesAreParsed() {
    assertEquals(
        PublishFailurePolicy.FAIL_REQUEST,
        PublishFailurePolicy.fromEnvironment(Map.of(PublishFailurePolicy.ENV, "fail-request")));
    assertEquals(
        PublishFailurePolicy.LOG_AND_CONTINUE,
        PublishFailurePolicy.fromEnvironment(
            Map.of(PublishFailurePolicy.ENV, " log-and-continue ")));
  }

  @Test
  void unknownPoliciesAreRejected() {
    for (String value : new String[] {"FAIL_REQUEST", "ignore", "log_and_continue"}) {
      IllegalStateException error =
          assertThrows(
              IllegalStateException.class,
              () -> PublishFailurePolicy.fromEnvironment(Map.of(PublishFailurePolicy.ENV, value)));
      assertTrue(error.getMessage().contains("fail-request, log-and-continue"));
    }
  }
}
