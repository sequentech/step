// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.events.Details;

class CustomEventListenerProviderTest {

  @Test
  void aPinLinkEntrySaysWhatHappened() {
    assertEquals(
        "pin_link opened",
        CustomEventListenerProvider.pinLinkBody(Map.of("pin_link_stage", "opened")));
    assertEquals(
        "pin_link refused wrong_username",
        CustomEventListenerProvider.pinLinkBody(
            Map.of("pin_link_stage", "refused", Details.REASON, "wrong_username")));
  }

  @Test
  void aPinLinkEntryOnlyTakesItsStageAndReason() {
    assertEquals(
        "pin_link revealed",
        CustomEventListenerProvider.pinLinkBody(
            Map.of("pin_link_stage", "revealed", Details.USERNAME, "someone")));
  }
}
