// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import jakarta.ws.rs.core.MultivaluedHashMap;
import java.net.URI;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class ReturnUrlTest {
  private static final URI LOGIN_ACTIONS = URI.create("https://kc/realms/r/login-actions");

  @Test
  void returnUrlPointsToTheRealmEndpointAndKeepsTheFlow() throws ScanovateException {
    URI actionUrl =
        URI.create(
            "https://kc/realms/r/login-actions/registration?session_code=c&execution=e&client_id=voting-portal&tab_id=t&client_data=d%3D");

    assertEquals(
        URI.create(
            "https://kc/realms/r/identity-verification/return?flow=registration&session_code=c&execution=e&client_id=voting-portal&tab_id=t&client_data=d%3D"),
        ReturnUrl.fromActionUrl(actionUrl));
  }

  @Test
  void returnUrlRejectsUrlsOutsideLoginActions() {
    assertThrows(
        ScanovateException.class,
        () -> ReturnUrl.fromActionUrl(URI.create("https://kc/realms/r/account")));
  }

  @Test
  void actionUrlDropsTheBTrustTokenAndUnknownParameters() {
    MultivaluedHashMap<String, String> params = new MultivaluedHashMap<>();
    params.add(ReturnUrl.FLOW_PARAM, "registration");
    params.add("session_code", "c");
    params.add("execution", "e");
    params.add("client_id", "voting-portal");
    params.add("tab_id", "t");
    params.add("client_data", "d=");
    params.add(ScanovateAuthenticator.PROCESS_ID_QUERY_PARAM, "proc-1");
    params.add("token", "btrust-session-token");
    params.add("redirect_uri", "https://evil.example.com");

    assertEquals(
        Optional.of(
            URI.create(
                "https://kc/realms/r/login-actions/registration?session_code=c&execution=e&client_id=voting-portal&tab_id=t&client_data=d%3D&processId=proc-1")),
        ReturnUrl.toActionUrl(LOGIN_ACTIONS, params));
  }

  @Test
  void actionUrlSupportsEveryLoginActionsFlow() {
    for (String flow :
        new String[] {
          "authenticate",
          "registration",
          "reset-credentials",
          "first-broker-login",
          "post-broker-login"
        }) {
      MultivaluedHashMap<String, String> params = new MultivaluedHashMap<>();
      params.add(ReturnUrl.FLOW_PARAM, flow);
      assertEquals(
          Optional.of(URI.create("https://kc/realms/r/login-actions/" + flow)),
          ReturnUrl.toActionUrl(LOGIN_ACTIONS, params));
    }
  }

  @Test
  void actionUrlRejectsUnknownOrMissingFlows() {
    for (String flow :
        new String[] {"../../evil", "https://evil.example.com", "", "Registration"}) {
      MultivaluedHashMap<String, String> params = new MultivaluedHashMap<>();
      params.add(ReturnUrl.FLOW_PARAM, flow);
      assertTrue(ReturnUrl.toActionUrl(LOGIN_ACTIONS, params).isEmpty(), flow);
    }
    assertTrue(ReturnUrl.toActionUrl(LOGIN_ACTIONS, new MultivaluedHashMap<>()).isEmpty());
  }
}
