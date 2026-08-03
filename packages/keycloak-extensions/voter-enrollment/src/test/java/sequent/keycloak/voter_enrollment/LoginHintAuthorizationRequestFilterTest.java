// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.container.ContainerRequestContext;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriInfo;
import java.net.URI;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

class LoginHintAuthorizationRequestFilterTest {

  @Test
  void acceptsFiveScalarHintsAndDecodesReservedAndUnicodeValues() {
    String query =
        "client_id=voting-portal"
            + "&login_hint__field1=one"
            + "&login_hint__field2=two"
            + "&login_hint__field3=three"
            + "&login_hint__field4=a%26b%3Dc"
            + "&login_hint__field5=Jos%C3%A9";

    assertEquals(
        Map.of(
            "field1", "one",
            "field2", "two",
            "field3", "three",
            "field4", "a&b=c",
            "field5", "José"),
        LoginHintAuthorizationRequestFilter.validateRawQuery(query));
  }

  @Test
  void rejectsTheCompleteSetForPollutionBoundsAndMalformedEncoding() {
    String sixHints =
        "login_hint__f0=v0&login_hint__f1=v1&login_hint__f2=v2"
            + "&login_hint__f3=v3&login_hint__f4=v4&login_hint__f5=v5";
    String overlongName = "login_hint__" + "a".repeat(129) + "=value";
    String overlongValue = "login_hint__username=" + "a".repeat(256);

    for (String query :
        new String[] {
          sixHints,
          "login_hint__username=",
          "login_hint__username=%20",
          "login_hint__username=%C2%A0",
          "login_hint__username=%EF%BB%BF",
          "login_hint__username=first&login_hint__username=second",
          "login_hint__user%6Eame=first&login_hint__username=second",
          "login_hint__first%20name=value",
          overlongName,
          overlongValue,
          "login_hint__username=user%ZZexample",
          "login_hint__username=%E0%A4%A"
        }) {
      assertThrows(
          IllegalArgumentException.class,
          () -> LoginHintAuthorizationRequestFilter.validateRawQuery(query),
          query);
    }
  }

  @Test
  void matchesOnlyTheTwoRealmAuthorizationEndpoints() {
    assertTrue(
        LoginHintAuthorizationRequestFilter.isAuthorizationEndpoint(
            "/realms/example/protocol/openid-connect/auth"));
    assertTrue(
        LoginHintAuthorizationRequestFilter.isAuthorizationEndpoint(
            "/keycloak/realms/example/protocol/openid-connect/registrations/"));
    assertTrue(
        LoginHintAuthorizationRequestFilter.isAuthorizationEndpoint(
            "/realms/example/protocol/openid-connect/auth;matrix=1"));
    assertTrue(
        LoginHintAuthorizationRequestFilter.isAuthorizationEndpoint(
            "/realms/example/protocol;v=1/openid-connect/registrations;matrix=1"));
    assertFalse(
        LoginHintAuthorizationRequestFilter.isAuthorizationEndpoint(
            "/realms/example/protocol/openid-connect/token"));
    assertFalse(
        LoginHintAuthorizationRequestFilter.isAuthorizationEndpoint(
            "/admin/realms/example/protocol/openid-connect/auth"));
    assertFalse(
        LoginHintAuthorizationRequestFilter.isAuthorizationEndpoint(
            "/admin;matrix=1/realms/example/protocol/openid-connect/auth;matrix=1"));
  }

  @Test
  void invalidRequestAbortsWithAValueFreeOAuthError() {
    ContainerRequestContext context = mock(ContainerRequestContext.class);
    UriInfo uriInfo = mock(UriInfo.class);
    when(context.getUriInfo()).thenReturn(uriInfo);
    when(uriInfo.getRequestUri())
        .thenReturn(
            URI.create(
                "https://id.example/realms/test/protocol/openid-connect/auth;matrix=1"
                    + "?login_hint__username=private-value"
                    + "&login_hint__username=other-private-value"));

    new LoginHintAuthorizationRequestFilter().filter(context);

    ArgumentCaptor<Response> response = ArgumentCaptor.forClass(Response.class);
    verify(context).abortWith(response.capture());
    assertEquals(Response.Status.BAD_REQUEST.getStatusCode(), response.getValue().getStatus());
    assertEquals(
        Map.of(
            "error", LoginHintAuthorizationRequestFilter.INVALID_REQUEST_ERROR,
            "error_description", LoginHintAuthorizationRequestFilter.INVALID_REQUEST_DESCRIPTION),
        response.getValue().getEntity());
    assertFalse(response.getValue().getEntity().toString().contains("private-value"));
  }

  @Test
  void validAndUnrelatedRequestsContinueUnmodified() {
    ContainerRequestContext context = mock(ContainerRequestContext.class);
    UriInfo uriInfo = mock(UriInfo.class);
    when(context.getUriInfo()).thenReturn(uriInfo);
    when(uriInfo.getRequestUri())
        .thenReturn(
            URI.create(
                "https://id.example/realms/test/protocol/openid-connect/auth"
                    + "?state=oidc-state&nonce=oidc-nonce&code_challenge=pkce"
                    + "&login_hint__username=voter"));

    new LoginHintAuthorizationRequestFilter().filter(context);
    verify(context, never()).abortWith(org.mockito.ArgumentMatchers.any());

    when(uriInfo.getRequestUri())
        .thenReturn(
            URI.create(
                "https://id.example/realms/test/protocol/openid-connect/token"
                    + "?login_hint__username=&login_hint__username=duplicate"));
    new LoginHintAuthorizationRequestFilter().filter(context);
    verify(context, never()).abortWith(org.mockito.ArgumentMatchers.any());
  }
}
