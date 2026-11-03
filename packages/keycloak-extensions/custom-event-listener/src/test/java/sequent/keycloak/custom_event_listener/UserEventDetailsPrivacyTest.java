// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;

import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.events.Event;

class UserEventDetailsPrivacyTest {
  @Test
  void scanovateBarcodeExceptionOnlyAcceptsBoundedCheckValues() {
    Event event = new Event();
    event.setError("scanovate_verification_failed");
    for (String value : java.util.List.of("true", "false", "missing", "invalid")) {
      event.setDetails(
          Map.of("scanovate_check_barcode_valid", value, "access_token", "synthetic-secret"));
      assertEquals(
          Map.of(
              "error",
              "scanovate_verification_failed",
              "details",
              Map.of("scanovate_check_barcode_valid", value)),
          UserEventDetails.of(event, UserEventDetails.Account.KNOWN));
    }
    event.setDetails(Map.of("scanovate_check_barcode_valid", "synthetic-secret"));
    assertEquals(
        Map.of(), UserEventDetails.of(event, UserEventDetails.Account.KNOWN).get("details"));
    event.setError("invalid_user_credentials");
    event.setDetails(Map.of("scanovate_check_barcode_valid", "true"));
    assertEquals(
        Map.of(), UserEventDetails.of(event, UserEventDetails.Account.KNOWN).get("details"));
  }

  @Test
  void authenticationAuditDropsNetworkDataBeforeTheDurableQueue() {
    Event event = new Event();
    event.setIpAddress("198.51.100.7");
    event.setError("invalid_user_credentials");
    event.setDetails(
        Map.of(
            "auth_method",
            "openid-connect",
            "userAgent",
            "private browser",
            "country",
            "TR",
            "ip-address",
            "198.51.100.9",
            "reason",
            "from 2001:db8::1"));
    Map<String, Object> result = UserEventDetails.of(event, UserEventDetails.Account.KNOWN);
    assertEquals("invalid_user_credentials", result.get("error"));
    assertEquals("openid-connect", result.get("auth_method"));
    String text = result.toString();
    assertFalse(text.contains("198.51.100"));
    assertFalse(text.contains("2001:db8"));
    assertFalse(text.contains("private browser"));
    assertFalse(text.contains("country"));
    assertFalse(result.containsKey("ip_address"));
  }

  @Test
  void freeformErrorsAreReplacedWithAnAuditOutcomeCode() {
    assertEquals(
        "authentication_error", UserEventDetails.safeError("Login from 198.51.100.7 failed"));
    assertEquals("user_not_found", UserEventDetails.safeError("user_not_found"));
    assertNull(UserEventDetails.safeError(null));
  }
}
