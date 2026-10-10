// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.RETURNS_DEEP_STUBS;
import static org.mockito.Mockito.doNothing;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.spy;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.MockedStatic;
import sequent.keycloak.harvest.HarvestEndpoint;
import sequent.keycloak.harvest.HarvestTlsPolicy;

class LookupAndUpdateUserTest {

  private static final String REALM_ID = "realm-id";
  private static final String EVENT_REALM_NAME =
      "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-5e3d1c8f-2f1b-4f57-9d6e-0a5f7d7c2b11";
  private static final String VERIFIED_USER_ID = "verified-user-id";
  private static final String ID_CARD_NUMBER = "sequent.read-only.id-card-number";
  private static final String DATE_OF_BIRTH = "dateOfBirth";
  private static final Map<String, String> APPLICANT_DATA =
      Map.of(ID_CARD_NUMBER, "1234-5678", DATE_OF_BIRTH, "1980-01-31");
  private static final String VERIFY_APPLICATION_PATH = "/verify-application";
  private static final String ACCEPTED_RESPONSE =
      "{\"user_id\": \""
          + VERIFIED_USER_ID
          + "\", \"application_status\": \"ACCEPTED\", \"application_type\": \"AUTOMATIC\","
          + " \"mismatches\": null, \"fields_match\": null, \"attributes_unset\": null,"
          + " \"rejection_reason\": null, \"rejection_message\": null}";

  private HttpServer harvest;
  private final AtomicInteger harvestRequests = new AtomicInteger();

  @AfterEach
  void stopHarvest() {
    if (harvest != null) {
      harvest.stop(0);
    }
  }

  @Test
  void verifyApplicationUrlUsesHarvestUrl() throws IOException {
    Map<String, String> environment =
        Map.of(
            HarvestEndpoint.ENV_HARVEST_URL, "https://harvest.internal:8443",
            HarvestEndpoint.ENV_HARVEST_DOMAIN, "harvest:8400",
            HarvestEndpoint.ENV_HARVEST_TLS_POLICY, HarvestTlsPolicy.REQUIRE_TLS.name());

    assertEquals(
        "https://harvest.internal:8443/verify-application",
        LookupAndUpdateUser.verifyApplicationUrl(environment::get));
  }

  @Test
  void verifyApplicationUrlFallsBackToPlainHttpHarvestDomain() throws IOException {
    Map<String, String> environment = Map.of(HarvestEndpoint.ENV_HARVEST_DOMAIN, "harvest:8400");

    assertEquals(
        "http://harvest:8400/verify-application",
        LookupAndUpdateUser.verifyApplicationUrl(environment::get));
  }

  @Test
  void verifyApplicationUrlRejectsPlainHttpWhenTlsIsRequired() {
    Map<String, String> environment =
        Map.of(
            HarvestEndpoint.ENV_HARVEST_DOMAIN,
            "harvest:8400",
            HarvestEndpoint.ENV_HARVEST_TLS_POLICY,
            HarvestTlsPolicy.REQUIRE_TLS.name());

    assertThrows(
        IllegalStateException.class,
        () -> LookupAndUpdateUser.verifyApplicationUrl(environment::get));
  }

  @Test
  void verifyApplicationUrlRequiresConfiguredHarvest() {
    Map<String, String> environment = Map.of();

    assertThrows(
        IOException.class, () -> LookupAndUpdateUser.verifyApplicationUrl(environment::get));
  }

  /**
   * An accepted application whose returned user differs from the applicant data on the search
   * attributes ends in a failure challenge, without setting, updating or logging in that user.
   */
  @Test
  void refusesAcceptedApplicationWhoseUserDoesNotMatchApplicant() throws IOException {
    UserModel verifiedUser =
        userWithAttributes(Map.of(ID_CARD_NUMBER, "9999-0000", DATE_OF_BIRTH, "1990-12-01"));
    AuthenticationFlowContext context = enrollmentContext(verifiedUser);

    runAuthenticate(context);

    verify(context).failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any());
    verify(context, never()).setUser(any());
    verify(context, never()).success();
    verify(verifiedUser, never()).credentialManager();
    assertEquals(1, harvestRequests.get());
  }

  /** An accepted application whose returned user matches the applicant data still logs in. */
  @Test
  void logsInAcceptedApplicationUserMatchingApplicant() throws IOException {
    UserModel verifiedUser = userWithAttributes(APPLICANT_DATA);
    AuthenticationFlowContext context = enrollmentContext(verifiedUser);

    runAuthenticate(context);

    verify(context).setUser(verifiedUser);
    verify(context).success();
    verify(context, never()).failureChallenge(any(), any());
    assertEquals(1, harvestRequests.get());
  }

  /**
   * Runs the authenticator against a local harvest stub that accepts the application, with the
   * service token request and the applicant data lookup stubbed out.
   */
  private void runAuthenticate(AuthenticationFlowContext context) throws IOException {
    startHarvest();
    LookupAndUpdateUser authenticator = spy(new LookupAndUpdateUser());
    doNothing().when(authenticator).authenticate(anyString());
    Map<String, String> environment =
        Map.of(
            HarvestEndpoint.ENV_HARVEST_URL,
            "http://"
                + harvest.getAddress().getHostString()
                + ":"
                + harvest.getAddress().getPort());
    authenticator.environment = environment::get;

    try (MockedStatic<Utils> utils = mockStatic(Utils.class)) {
      utils.when(() -> Utils.buildApplicantData(any(), any())).thenReturn(APPLICANT_DATA);

      authenticator.authenticate(context);
    }
  }

  /** Starts a harvest stub whose verify-application endpoint returns {@link #ACCEPTED_RESPONSE}. */
  private void startHarvest() throws IOException {
    harvest = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
    harvest.createContext(
        VERIFY_APPLICATION_PATH,
        exchange -> {
          harvestRequests.incrementAndGet();
          byte[] body = ACCEPTED_RESPONSE.getBytes(StandardCharsets.UTF_8);
          exchange.getResponseHeaders().add("Content-Type", "application/json");
          exchange.sendResponseHeaders(200, body.length);
          try (OutputStream out = exchange.getResponseBody()) {
            out.write(body);
          }
        });
    harvest.start();
  }

  /** Returns a user whose first value of each given attribute is the given value. */
  private static UserModel userWithAttributes(Map<String, String> attributes) {
    UserModel user = mock(UserModel.class, RETURNS_DEEP_STUBS);
    attributes.forEach((name, value) -> when(user.getFirstAttribute(name)).thenReturn(value));
    return user;
  }

  /**
   * Builds an enrollment flow context for an event realm with auto-login on, where the realm user
   * named in {@link #ACCEPTED_RESPONSE} is {@code verifiedUser}.
   */
  private static AuthenticationFlowContext enrollmentContext(UserModel verifiedUser) {
    RealmModel realm = mock(RealmModel.class, RETURNS_DEEP_STUBS);
    when(realm.getId()).thenReturn(REALM_ID);
    when(realm.getName()).thenReturn(EVENT_REALM_NAME);

    KeycloakSession session = mock(KeycloakSession.class, RETURNS_DEEP_STUBS);
    when(session.realms().getRealm(REALM_ID)).thenReturn(realm);
    when(session.users().getUserById(realm, VERIFIED_USER_ID)).thenReturn(verifiedUser);

    AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
    when(config.getConfig())
        .thenReturn(
            Map.of(
                LookupAndUpdateUser.SEARCH_ATTRIBUTES,
                ID_CARD_NUMBER + "," + DATE_OF_BIRTH,
                LookupAndUpdateUser.AUTO_LOGIN,
                Boolean.TRUE.toString()));

    AuthenticationSessionModel authSession =
        mock(AuthenticationSessionModel.class, RETURNS_DEEP_STUBS);
    when(authSession.getAuthNote(RegistrationPage.FIELD_PASSWORD)).thenReturn("password");

    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class, RETURNS_DEEP_STUBS);
    when(context.getRealm()).thenReturn(realm);
    when(context.getSession()).thenReturn(session);
    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    return context;
  }
}
