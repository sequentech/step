// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;
import static org.mockito.Mockito.when;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.Base64;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.events.Event;
import org.keycloak.events.EventType;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.events.admin.AuthDetails;
import org.keycloak.events.admin.OperationType;
import org.keycloak.events.admin.ResourceType;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakTransaction;
import org.keycloak.models.KeycloakTransactionManager;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RealmProvider;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.mockito.ArgumentCaptor;

class CustomEventListenerProviderEventsTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String ELECTION_EVENT = "6f1c3a6e-2a61-4d6b-9a52-3f0f8a3c2b10";
  private static final String TENANT_REALM = "tenant-" + TENANT;
  private static final String EVENT_REALM = TENANT_REALM + "-event-" + ELECTION_EVENT;
  private static final String USER_ID = "0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09";
  private static final long TIME = 1778511600123L;

  private final ObjectMapper om = new ObjectMapper();
  private KeycloakSession session;
  private RealmProvider realms;
  private UserProvider users;
  private QueueSender sender;
  private CustomEventListenerProvider provider;

  @BeforeEach
  void setUp() {
    session = mock(KeycloakSession.class);
    realms = mock(RealmProvider.class);
    users = mock(UserProvider.class);
    sender = mock(QueueSender.class);
    KeycloakTransactionManager transaction = mock(KeycloakTransactionManager.class);
    when(session.realms()).thenReturn(realms);
    when(session.users()).thenReturn(users);
    when(session.getTransactionManager()).thenReturn(transaction);
    when(transaction.isActive()).thenReturn(false);
    provider =
        new CustomEventListenerProvider(
            session, new PgmqEventPublisher(sender, PublishFailurePolicy.FAIL_REQUEST));
  }

  @Test
  void scanovateContractSurvivesTheRealListenerAndQueueEnvelope() throws Exception {
    JsonNode fixture =
        om.readTree(getClass().getResourceAsStream("/scanovate-failure-contract.json"));
    JsonNode expected = fixture.get("queue_input").deepCopy();
    String tenant = expected.get("tenant_id").asText();
    String election = expected.get("election_event_id").asText();
    String user = expected.get("user_id").asText();
    String name = "tenant-" + tenant + "-event-" + election;
    account(realm(name), user, "synthetic-voter");
    Event event = event(name, EventType.REGISTER_ERROR, user, "scanovate_verification_failed");
    Map<String, String> details = new HashMap<>();
    fixture
        .get("producer_details")
        .fields()
        .forEachRemaining(field -> details.put(field.getKey(), field.getValue().asText()));
    // Even a legacy producer's extra fields must not become a new durable disclosure.
    details.put("email", "person@example.invalid");
    details.put("passport", "P123456789");
    details.put("scanovate_raw_response", "synthetic secret");
    details.put("code_id", "synthetic-session");
    details.put("scanovate_http_status", "999");
    event.setDetails(details);
    provider.onEvent(event);
    JsonNode actual = publishedInput();
    ((com.fasterxml.jackson.databind.node.ObjectNode) actual)
        .set("body", om.readTree(actual.get("body").asText()));
    ((com.fasterxml.jackson.databind.node.ObjectNode) expected)
        .set("body", om.readTree(expected.get("body").asText()));
    assertEquals(expected, actual);
    assertEquals("missing", actual.at("/body/details/scanovate_check_barcode_valid").asText());
    for (String forbidden :
        java.util.List.of(
            "person@", "P123456789", "synthetic secret", "synthetic-session", "999")) {
      assertFalse(actual.toString().contains(forbidden), forbidden);
    }
  }

  @Test
  void successfulSignInCaptureRunsAfterCommitAndNeverChangesTheAuditPayload() throws Exception {
    SigninNetworkDispatcher capture = mock(SigninNetworkDispatcher.class);
    provider =
        new CustomEventListenerProvider(
            session, new PgmqEventPublisher(sender, PublishFailurePolicy.FAIL_REQUEST), capture);
    account(realm(EVENT_REALM), USER_ID, "voter");
    KeycloakTransactionManager transaction = session.getTransactionManager();
    when(transaction.isActive()).thenReturn(true);
    Event event = event(EVENT_REALM, EventType.LOGIN, USER_ID, null);
    event.setDetails(Map.of("country", "ES", "user_agent", "Synthetic private browser"));
    provider.onEvent(event);
    verifyNoInteractions(capture, sender);
    ArgumentCaptor<KeycloakTransaction> audit = ArgumentCaptor.forClass(KeycloakTransaction.class);
    verify(transaction).enlistPrepare(audit.capture());
    audit.getValue().begin();
    audit.getValue().commit();
    String ordinary = publishedInput().toString();
    assertFalse(ordinary.contains("10.0.0.7"));
    assertFalse(ordinary.contains("Synthetic private browser"));
    assertFalse(ordinary.contains("country"));
    ArgumentCaptor<KeycloakTransaction> after = ArgumentCaptor.forClass(KeycloakTransaction.class);
    verify(transaction).enlistAfterCompletion(after.capture());
    after.getValue().begin();
    after.getValue().commit();
    ArgumentCaptor<SigninNetworkSender.Evidence> snapshot =
        ArgumentCaptor.forClass(SigninNetworkSender.Evidence.class);
    verify(capture).submit(snapshot.capture());
    assertEquals(USER_ID, snapshot.getValue().voterId());
    assertEquals("10.0.0.7", snapshot.getValue().ip());
  }

  @Test
  void rolledBackSignInDoesNotSendForensicAttribution() {
    SigninNetworkDispatcher capture = mock(SigninNetworkDispatcher.class);
    provider =
        new CustomEventListenerProvider(
            session, new PgmqEventPublisher(sender, PublishFailurePolicy.FAIL_REQUEST), capture);
    account(realm(EVENT_REALM), USER_ID, "voter");
    KeycloakTransactionManager transaction = session.getTransactionManager();
    when(transaction.isActive()).thenReturn(true);
    provider.onEvent(event(EVENT_REALM, EventType.LOGIN, USER_ID, null));
    ArgumentCaptor<KeycloakTransaction> after = ArgumentCaptor.forClass(KeycloakTransaction.class);
    verify(transaction).enlistAfterCompletion(after.capture());
    after.getValue().begin();
    after.getValue().rollback();
    verifyNoInteractions(capture, sender);
  }

  /** A realm whose id is not its name, as in Keycloak. */
  private RealmModel realm(String name) {
    RealmModel realm = mock(RealmModel.class);
    when(realm.getName()).thenReturn(name);
    when(realms.getRealm("id-of-" + name)).thenReturn(realm);
    return realm;
  }

  private void account(RealmModel realm, String userId, String username) {
    UserModel user = mock(UserModel.class);
    when(user.getUsername()).thenReturn(username);
    when(users.getUserById(realm, userId)).thenReturn(user);
  }

  private Event event(String realmName, EventType type, String userId, String error) {
    Event event = new Event();
    event.setRealmId("id-of-" + realmName);
    event.setType(type);
    event.setUserId(userId);
    event.setError(error);
    event.setClientId("admin-portal");
    event.setIpAddress("10.0.0.7");
    event.setTime(TIME);
    return event;
  }

  private JsonNode publishedInput() throws Exception {
    ArgumentCaptor<String> payload = ArgumentCaptor.forClass(String.class);
    verify(sender).send(eq(PgmqEventPublisher.QUEUE), payload.capture());
    JsonNode envelope = om.readTree(payload.getValue());
    assertEquals("enqueue_electoral_log_event", envelope.at("/headers/task").asText());
    JsonNode message = om.readTree(Base64.getDecoder().decode(envelope.get("body").asText()));
    return message.get(1).get("input");
  }

  @Test
  void aTenantSignInIsPublishedWithoutAnElectionEvent() throws Exception {
    account(realm(TENANT_REALM), USER_ID, "admin");
    Event event = event(TENANT_REALM, EventType.LOGIN, USER_ID, null);
    Map<String, String> details = new HashMap<>();
    details.put("auth_method", "openid-connect");
    details.put("auth_type", "code");
    details.put("credential_type", "otp");
    details.put("identity_provider", "corporate-sso");
    details.put("redirect_uri", "https://admin.example.com/");
    details.put("consent", "no_consent_required");
    details.put("code_id", "f3a1c0de");
    details.put("username", "admin");
    event.setDetails(details);

    provider.onEvent(event);

    assertEquals(
        om.readTree(
            """
            {"election_event_id": "",
             "message_type": "LOGIN",
             "body": "null",
             "user_id": "0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09",
             "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
             "username": "admin",
             "event_time_ms": 1778511600123,
             "details": {
               "client_id": "admin-portal",
               "error": null,
               "auth_method": "openid-connect",
               "auth_type": "code",
               "credential_type": "otp",
               "identity_provider": "corporate-sso",
               "details": {
                 "consent": "no_consent_required",
                 "redirect_uri": "https://admin.example.com/",
                 "username": "admin"}}}
            """),
        publishedInput());
  }

  @Test
  void aTenantSignInErrorForAnUnknownAccountDoesNotCarryWhatWasTyped() throws Exception {
    realm(TENANT_REALM);
    Event event = event(TENANT_REALM, EventType.LOGIN_ERROR, null, "user_not_found");
    Map<String, String> details = new HashMap<>();
    details.put("auth_method", "openid-connect");
    details.put("auth_type", "code");
    details.put("redirect_uri", "https://admin.example.com/");
    details.put("code_id", "f3a1c0de");
    details.put("username", "hunter2-typed-in-the-wrong-box");
    details.put("email", "typed@example.com");
    event.setDetails(details);

    provider.onEvent(event);

    JsonNode input = publishedInput();
    assertEquals(
        om.readTree(
            """
            {"election_event_id": "",
             "message_type": "LOGIN_ERROR",
             "body": "user_not_found",
             "user_id": "null",
             "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
             "username": "null",
             "event_time_ms": 1778511600123,
             "details": {
               "client_id": "admin-portal",
               "error": "user_not_found",
               "auth_method": "openid-connect",
               "auth_type": "code",
               "credential_type": null,
               "unknown_account": true,
               "details": {"redirect_uri": "https://admin.example.com/"}}}
            """),
        input);
    assertFalse(input.toString().contains("typed"));
  }

  @Test
  void anAccountThatNoLongerExistsIsAnUnknownAccount() throws Exception {
    realm(TENANT_REALM);
    Event event = event(TENANT_REALM, EventType.LOGIN_ERROR, USER_ID, "user_not_found");
    event.setDetails(Map.of("username", "typed"));

    provider.onEvent(event);

    JsonNode input = publishedInput();
    assertEquals(USER_ID, input.get("user_id").asText());
    assertEquals("null", input.get("username").asText());
    assertTrue(input.at("/details/unknown_account").asBoolean());
    assertTrue(input.at("/details/details").isEmpty());
  }

  @Test
  void aTenantLockoutIsPublished() throws Exception {
    account(realm(TENANT_REALM), USER_ID, "admin");
    Event event = event(TENANT_REALM, EventType.USER_DISABLED_BY_TEMPORARY_LOCKOUT, USER_ID, null);
    event.setClientId(null);
    event.setDetails(Map.of("reason", "brute_force_attack detected", "num_failures", "5"));

    provider.onEvent(event);

    JsonNode input = publishedInput();
    assertEquals("USER_DISABLED_BY_TEMPORARY_LOCKOUT", input.get("message_type").asText());
    assertEquals("", input.get("election_event_id").asText());
    assertEquals(TENANT, input.get("tenant_id").asText());
    assertEquals("admin", input.get("username").asText());
    assertEquals(
        om.readTree(
            """
            {"client_id": null,
             "error": null,
             "auth_method": null,
             "auth_type": null,
             "credential_type": null,
             "details": {"reason": "brute_force_attack detected", "num_failures": "5"}}
            """),
        input.get("details"));
  }

  @Test
  void secretsInTheDetailsAreNeverPublished() throws Exception {
    account(realm(TENANT_REALM), USER_ID, "admin");
    Event event = event(TENANT_REALM, EventType.UPDATE_PASSWORD, USER_ID, null);
    Map<String, String> details = new HashMap<>();
    details.put("new_password", "hunter2");
    details.put("client_secret", "s3cret");
    details.put("refresh_token_id", "token-id");
    details.put("Token", "eyJ");
    details.put("code", "123456");
    details.put("code_id", "f3a1c0de");
    details.put("otp_code", "654321");
    details.put("credential_type", "password");
    details.put("custom_required_action", "UPDATE_PASSWORD");
    event.setDetails(details);

    provider.onEvent(event);

    assertEquals(
        om.readTree(
            """
            {"client_id": "admin-portal",
             "error": null,
             "auth_method": null,
             "auth_type": null,
             "credential_type": "password",
             "details": {"custom_required_action": "UPDATE_PASSWORD"}}
            """),
        publishedInput().get("details"));
  }

  @Test
  void theTenantRealmDropsMachineNoise() {
    account(realm(TENANT_REALM), USER_ID, "admin");
    for (EventType type :
        new EventType[] {
          EventType.REFRESH_TOKEN,
          EventType.CODE_TO_TOKEN,
          EventType.CLIENT_LOGIN,
          EventType.INTROSPECT_TOKEN,
          EventType.USER_INFO_REQUEST,
          EventType.TOKEN_EXCHANGE,
          EventType.PERMISSION_TOKEN
        }) {
      provider.onEvent(event(TENANT_REALM, type, USER_ID, null));
    }

    verifyNoInteractions(sender);
  }

  /** The details the authenticator gives an event when it sends a message to the account. */
  private static Map<String, String> sentMessage() {
    Map<String, String> details = new HashMap<>();
    details.put("type", "communications");
    details.put("msgBody", "Your code is ******");
    details.put("auth_method", "openid-connect");
    return details;
  }

  @Test
  void aMessageSentToATenantAccountIsPublishedWithoutItsText() throws Exception {
    account(realm(TENANT_REALM), USER_ID, "admin");
    // Whatever step sent it: a successful event of a type the tenant log otherwise leaves out.
    Event event = event(TENANT_REALM, EventType.CODE_TO_TOKEN, USER_ID, null);
    event.setDetails(sentMessage());

    provider.onEvent(event);

    JsonNode input = publishedInput();
    assertEquals("", input.get("election_event_id").asText());
    assertEquals("communications ", input.get("body").asText());
    assertEquals(
        om.readTree(
            """
            {"client_id": "admin-portal",
             "error": null,
             "auth_method": "openid-connect",
             "auth_type": null,
             "credential_type": null,
             "details": {"type": "communications"}}
            """),
        input.get("details"));
    assertFalse(input.toString().contains("Your code"), input.toString());
  }

  @Test
  void aMessageSentToAnElectionEventAccountKeepsItsTextInTheBodyOnly() throws Exception {
    account(realm(EVENT_REALM), USER_ID, "voter");
    Event event = event(EVENT_REALM, EventType.LOGIN, USER_ID, null);
    event.setDetails(sentMessage());

    provider.onEvent(event);

    JsonNode input = publishedInput();
    assertEquals("communications Your code is ******", input.get("body").asText());
    assertEquals(om.readTree("{\"type\": \"communications\"}"), input.at("/details/details"));
  }

  @Test
  void theTenantRealmKeepsEveryError() throws Exception {
    account(realm(TENANT_REALM), USER_ID, "admin");
    EventType[] errors = {
      EventType.LOGIN_ERROR, EventType.REFRESH_TOKEN_ERROR, EventType.CLIENT_LOGIN_ERROR
    };
    for (EventType type : errors) {
      provider.onEvent(event(TENANT_REALM, type, USER_ID, "invalid_user_credentials"));
    }

    verify(sender, times(errors.length)).send(eq(PgmqEventPublisher.QUEUE), anyString());
  }

  @Test
  void everyEventTypeKeptForTheTenantRealmExistsInKeycloak() {
    for (String name : TenantRealmEvents.KEPT_WHEN_SUCCESSFUL) {
      assertEquals(name, EventType.valueOf(name).name());
      assertFalse(name.endsWith("_ERROR"), name);
    }
  }

  @Test
  void theMasterRealmPublishesNothing() {
    account(realm("master"), USER_ID, "admin");

    provider.onEvent(event("master", EventType.LOGIN, USER_ID, null));
    provider.onEvent(adminEvent("master", "{\"enabled\":true}"), true);

    verifyNoInteractions(sender);
  }

  @Test
  void anEventRealmStillPublishesEverythingWithTheFieldsItAlwaysSent() throws Exception {
    realm(EVENT_REALM);
    Event event = event(EVENT_REALM, EventType.REFRESH_TOKEN, null, null);
    event.setDetails(Map.of("username", "voter@example.com", "refresh_token_id", "token-id"));

    provider.onEvent(event);

    JsonNode input = publishedInput();
    assertEquals(ELECTION_EVENT, input.get("election_event_id").asText());
    assertEquals("REFRESH_TOKEN", input.get("message_type").asText());
    assertEquals("null", input.get("body").asText());
    assertEquals("null", input.get("user_id").asText());
    assertEquals(TENANT, input.get("tenant_id").asText());
    assertEquals("voter@example.com", input.get("username").asText());
    assertEquals(TIME, input.get("event_time_ms").asLong());
    assertEquals(
        om.readTree(
            """
            {"client_id": "admin-portal",
             "error": null,
             "auth_method": null,
             "auth_type": null,
             "credential_type": null,
             "details": {"username": "voter@example.com"}}
            """),
        input.get("details"));
    assertEquals(8, input.size());
  }

  @Test
  void anEventRealmWithoutAUsernameStillSaysUnknown() throws Exception {
    realm(EVENT_REALM);

    provider.onEvent(event(EVENT_REALM, EventType.LOGIN_ERROR, null, "user_not_found"));

    JsonNode input = publishedInput();
    assertEquals("unknown", input.get("username").asText());
    assertEquals("user_not_found", input.get("body").asText());
    assertFalse(input.get("details").has("unknown_account"));
  }

  private AdminEvent adminEvent(String realmName, String representation) {
    AuthDetails actor = new AuthDetails();
    actor.setRealmId("id-of-" + TENANT_REALM);
    actor.setClientId("id-of-admin-portal");
    actor.setUserId(USER_ID);
    actor.setIpAddress("10.0.0.7");
    AdminEvent event = new AdminEvent();
    event.setRealmId("id-of-" + realmName);
    event.setAuthDetails(actor);
    event.setOperationType(OperationType.CREATE);
    event.setResourceType(ResourceType.USER);
    event.setResourcePath("users/3d2c1b0a-9f8e-4d7c-b6a5-0f1e2d3c4b5a");
    event.setRepresentation(representation);
    event.setTime(TIME);
    return event;
  }

  private void actorAccount() {
    RealmModel tenantRealm = realm(TENANT_REALM);
    account(tenantRealm, USER_ID, "admin");
    ClientModel client = mock(ClientModel.class);
    when(client.getClientId()).thenReturn("admin-portal");
    when(tenantRealm.getClientById("id-of-admin-portal")).thenReturn(client);
  }

  @Test
  void aTenantAdminEventIsPublishedWithItsActorAndARedactedRepresentation() throws Exception {
    actorAccount();

    provider.onEvent(
        adminEvent(
            TENANT_REALM,
            "{\"username\":\"auditor\",\"enabled\":true,"
                + "\"credentials\":[{\"type\":\"password\",\"value\":\"hunter2\"}]}"),
        true);

    assertEquals(
        om.readTree(
            """
            {"election_event_id": "",
             "message_type": "KEYCLOAK_ADMIN_EVENT",
             "body": "null",
             "user_id": "0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09",
             "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
             "username": "admin",
             "event_time_ms": 1778511600123,
             "admin_event": {
               "operation": "CREATE",
               "resource_type": "USER",
               "resource_path": "users/3d2c1b0a-9f8e-4d7c-b6a5-0f1e2d3c4b5a",
               "representation": {
                 "username": "auditor",
                 "enabled": true,
                 "credentials": [{"type": "password", "value": "**redacted**"}]},
               "error": null,
               "actor": {
                 "user_id": "0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09",
                 "username": "admin",
                 "client_id": "admin-portal",
                 "realm": "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5"}}}
            """),
        publishedInput());
  }

  @Test
  void anAdminEventOnAnEventRealmGoesToThatElectionEvent() throws Exception {
    actorAccount();
    realm(EVENT_REALM);
    AdminEvent event = adminEvent(EVENT_REALM, "plain text");
    event.setError("not_allowed");

    provider.onEvent(event, true);

    JsonNode input = publishedInput();
    assertEquals(ELECTION_EVENT, input.get("election_event_id").asText());
    assertEquals(TENANT, input.get("tenant_id").asText());
    assertEquals(AdminRepresentation.REDACTED, input.at("/admin_event/representation").asText());
    assertEquals("not_allowed", input.at("/admin_event/error").asText());
    assertEquals(TENANT_REALM, input.at("/admin_event/actor/realm").asText());
  }

  @Test
  void eventRealmAdminAuditDoesNotReintroduceNetworkAttribution() throws Exception {
    realm(EVENT_REALM);
    AdminEvent event =
        adminEvent(
            EVENT_REALM,
            "{\"enabled\":true,\"attributes\":{\"area\":[\"north\"],\"ip\":[\"198.51.100.7\"],\"user_agent\":[\"private-browser\"],\"country\":[\"PH\"]}}");
    provider.onEvent(event, true);
    JsonNode representation = publishedInput().at("/admin_event/representation");
    assertTrue(representation.path("enabled").asBoolean());
    assertEquals("north", representation.at("/attributes/area/0").asText());
    assertFalse(representation.toString().contains("198.51.100"));
    assertFalse(representation.toString().contains("private-browser"));
    assertFalse(representation.toString().contains("PH"));
    assertTrue(event.getRepresentation().contains("private-browser"));
  }

  @Test
  void anAdminEventWithoutDetailsOrAKnownActorCarriesWhatKeycloakGave() throws Exception {
    realm(TENANT_REALM);
    AdminEvent event = adminEvent(TENANT_REALM, "{\"enabled\":true}");
    event.getAuthDetails().setRealmId("id-of-master");

    provider.onEvent(event, false);

    JsonNode input = publishedInput();
    assertEquals(USER_ID, input.get("user_id").asText());
    assertEquals("null", input.get("username").asText());
    assertTrue(input.at("/admin_event/representation").isNull());
    assertEquals(
        om.readTree(
            """
            {"user_id": "0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09",
             "username": null,
             "client_id": "id-of-admin-portal",
             "realm": "id-of-master"}
            """),
        input.at("/admin_event/actor"));
  }

  @Test
  void aLongRepresentationIsPublishedAsItsHash() throws Exception {
    actorAccount();

    provider.onEvent(
        adminEvent(TENANT_REALM, "{\"description\":\"" + "a".repeat(70000) + "\"}"), true);

    JsonNode representation = publishedInput().at("/admin_event/representation");
    assertTrue(representation.get("truncated").asBoolean());
    assertEquals(64, representation.get("sha256").asText().length());
    assertEquals(2, representation.size());
  }
}
