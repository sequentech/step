// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.HttpHeaders;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.PathSegment;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import java.util.Arrays;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.KeycloakUriInfo;

class EnrollmentDocumentPolicyTest {
  @Test
  void preservesOtherPoliciesAndResponseMetadataWhileDenyingDisplayCapture() {
    String html = "<p>Enrollment</p>";
    Response response =
        Response.status(Response.Status.BAD_REQUEST)
            .type(MediaType.TEXT_HTML_TYPE)
            .entity(html)
            .header("permissions-policy", "camera=(self \"https://camera.example\")")
            .header(
                EnrollmentDocumentPolicy.HEADER,
                "microphone=(), display-capture=(self), geolocation=();report-to=\"g,events\"")
            .header(HttpHeaders.CACHE_CONTROL, "no-store")
            .header("Content-Security-Policy", "default-src 'self'")
            .build();

    Response protectedResponse = EnrollmentDocumentPolicy.protectHtml(response);
    String expected =
        "camera=(self \"https://camera.example\"), microphone=(), "
            + "geolocation=();report-to=\"g,events\", display-capture=()";
    assertEquals(expected, protectedResponse.getHeaderString(EnrollmentDocumentPolicy.HEADER));
    assertEquals(
        1, protectedResponse.getStringHeaders().get(EnrollmentDocumentPolicy.HEADER).size());
    assertEquals(response.getStatus(), protectedResponse.getStatus());
    assertEquals(response.getMediaType(), protectedResponse.getMediaType());
    assertEquals(html, protectedResponse.getEntity());
    assertEquals("no-store", protectedResponse.getHeaderString(HttpHeaders.CACHE_CONTROL));
    assertEquals(
        "default-src 'self'", protectedResponse.getHeaderString("Content-Security-Policy"));
    assertEquals(
        expected,
        EnrollmentDocumentPolicy.protectHtml(protectedResponse)
            .getHeaderString(EnrollmentDocumentPolicy.HEADER));
  }

  @Test
  void leavesUnrelatedDocumentsSharedByTheOtpAuthenticatorUnchanged() {
    Response codePage = Response.ok("<p>OTP</p>", MediaType.TEXT_HTML_TYPE).build();
    for (String path :
        List.of(
            "realms/example/login-actions/registration",
            "auth/realms/example/login-actions/registration/",
            "realms/example/protocol/openid-connect/registrations",
            "auth/realms/example/protocol/openid-connect/registrations/")) {
      assertEquals(
          EnrollmentDocumentPolicy.NO_DISPLAY_CAPTURE,
          EnrollmentDocumentPolicy.forRegistrationRoute(contextAt(path), codePage)
              .getHeaderString(EnrollmentDocumentPolicy.HEADER),
          path);
    }

    for (String path :
        List.of(
            "auth/realms/example/login-actions/authenticate",
            "realms/example/login-actions/reset-credentials",
            "realms/example/login-actions/registration-copy",
            "realms/example/login-actions/registration/extra",
            "realms//login-actions/registration",
            "realms/example/protocol/openid-connect/auth",
            "realms/example/protocol/saml/registrations",
            "other/example/login-actions/registration")) {
      assertSame(
          codePage, EnrollmentDocumentPolicy.forRegistrationRoute(contextAt(path), codePage), path);
    }
  }

  @Test
  void leavesRedirectsAndMachineResponsesUnchanged() {
    Response redirect = Response.seeOther(URI.create("/auth")).build();
    Response json = Response.ok("{}", MediaType.APPLICATION_JSON_TYPE).build();
    assertSame(redirect, EnrollmentDocumentPolicy.protectHtml(redirect));
    assertSame(json, EnrollmentDocumentPolicy.protectHtml(json));
  }

  @Test
  void malformedQuotedOrStructuredMembersCannotInvalidateTheResult() {
    assertEquals(
        "camera=(self), microphone=(), display-capture=()",
        EnrollmentDocumentPolicy.restrict(
            List.of(
                "camera=(self), geolocation=(\"https://not-closed",
                "broken=(self, microphone=(), junk=, display-capture=(self)")));
    assertEquals(
        "camera=*, interest-cohort=();report-to=\"a,\\\"b\", display-capture=()",
        EnrollmentDocumentPolicy.restrict(
            List.of("camera=*, interest-cohort=();report-to=\"a,\\\"b\", display-capture=*")));
  }

  private static AuthenticationFlowContext contextAt(String path) {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    var uri = uriAt(path);
    when(context.getUriInfo()).thenReturn(uri);
    return context;
  }

  static KeycloakUriInfo uriAt(String path) {
    KeycloakUriInfo uri = mock(KeycloakUriInfo.class);
    List<PathSegment> segments =
        Arrays.stream(path.split("/", -1))
            .map(
                element -> {
                  PathSegment segment = mock(PathSegment.class);
                  when(segment.getPath()).thenReturn(element);
                  return segment;
                })
            .toList();
    when(uri.getPathSegments()).thenReturn(segments);
    return uri;
  }
}
