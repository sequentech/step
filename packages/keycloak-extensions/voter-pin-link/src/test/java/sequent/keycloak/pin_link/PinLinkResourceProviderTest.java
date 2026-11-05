// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import static org.assertj.core.api.Assertions.assertThat;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.lenient;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.persistence.EntityManager;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.StreamingOutput;
import java.io.ByteArrayOutputStream;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import org.hibernate.Session;
import org.hibernate.jdbc.ReturningWork;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.common.ClientConnection;
import org.keycloak.connections.jpa.JpaConnectionProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.headers.SecurityHeadersOptions;
import org.keycloak.headers.SecurityHeadersProvider;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.KeycloakTransactionManager;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RealmProvider;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPAttributePermissions;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.services.managers.BruteForceProtector;
import org.keycloak.tracing.TracingProvider;
import org.keycloak.userprofile.UserProfileProvider;
import sequent.keycloak.custom_event_listener.RabbitMqEventPublisher;

/** Drives the HTTP layer against a real SQL database, with Keycloak's services mocked around it. */
class PinLinkResourceProviderTest {
  private static final String MASTER =
      "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
  private static final String TENANT = "11111111-1111-4111-8111-111111111111";
  private static final String EVENT = "22222222-2222-4222-8222-222222222222";
  private static final String VOTER_ID = "33333333-3333-4333-8333-333333333333";
  private static final String USERNAME = "member-0042";
  private static final String PIN = "123456789012";
  private static final String HASH_NAME = "pin-link-token-hash";
  private static final String FORM_PAGE = "<html><style id=\"x\"></style>form</html>";

  private final Map<String, String> realmAttributes = new HashMap<>();
  private final List<String> errors = new ArrayList<>();
  private final List<String> eventErrors = new ArrayList<>();
  private final UPConfig profile = new UPConfig();
  private final List<LogRecord> logs = new ArrayList<>();
  private final Handler logHandler =
      new Handler() {
        @Override
        public void publish(LogRecord record) {
          logs.add(record);
        }

        @Override
        public void flush() {}

        @Override
        public void close() {}
      };

  private String url;
  private Connection outer;
  private Connection admin;
  private String token;
  private KeycloakSession session;
  private LoginFormsProvider forms;
  private RealmModel realm;
  private UserModel user;
  private BruteForceProtector protector;
  private RabbitMqEventPublisher publisher;
  private PinLinkResourceProvider provider;
  private String revealPlaceholder;
  private boolean revealing;

  @BeforeEach
  void setUp() throws Exception {
    url =
        "jdbc:h2:mem:"
            + UUID.randomUUID()
            + ";MODE=PostgreSQL;NON_KEYWORDS=VALUE;DB_CLOSE_DELAY=-1";
    admin = DriverManager.getConnection(url);
    outer = DriverManager.getConnection(url);
    try (Statement statement = admin.createStatement()) {
      statement.execute(
          "CREATE TABLE user_entity (id VARCHAR(36) PRIMARY KEY, realm_id VARCHAR(36))");
      statement.execute(
          "CREATE TABLE user_attribute (id VARCHAR(36) PRIMARY KEY, user_id VARCHAR(36),"
              + " name VARCHAR(255), value VARCHAR(255))");
      statement.execute("INSERT INTO user_entity VALUES ('" + VOTER_ID + "', 'realm-id')");
    }
    token =
        PinLinkToken.derive(MASTER, TENANT, EVENT, VOTER_ID, PIN.getBytes(StandardCharsets.UTF_8));
    insertAttribute("pin", TestEnvelopes.encrypt(MASTER, TENANT, EVENT, VOTER_ID, "pin", PIN));
    insertAttribute(HASH_NAME, PinLinkToken.lookupHash(token));

    realmAttributes.put(PinLinkConfig.POLICY, "ONE_TIME_LINK");
    realmAttributes.put(PinLinkConfig.EXPIRES_AT, "2099-01-01T00:00:00Z");
    realmAttributes.put(PinLinkConfig.MAX_ATTEMPTS, "2");
    declarePin(Map.of("sequent.secret", "true"), Set.of("admin"));

    realm = mock(RealmModel.class);
    lenient().when(realm.getId()).thenReturn("realm-id");
    lenient().when(realm.getName()).thenReturn("tenant-" + TENANT + "-event-" + EVENT);
    lenient().when(realm.isBruteForceProtected()).thenReturn(true);
    lenient()
        .when(realm.getAttribute(anyString()))
        .thenAnswer(a -> realmAttributes.get(a.getArgument(0, String.class)));
    lenient()
        .when(realm.getBrowserSecurityHeaders())
        .thenReturn(Map.of("strictTransportSecurity", "max-age=31536000"));

    user = mock(UserModel.class);
    lenient().when(user.getId()).thenReturn(VOTER_ID);
    lenient().when(user.getUsername()).thenReturn(USERNAME);
    lenient().when(user.isEnabled()).thenReturn(true);

    protector = mock(BruteForceProtector.class);
    publisher = mock(RabbitMqEventPublisher.class);
    forms = mock(LoginFormsProvider.class);
    lenient()
        .when(forms.setAttribute(anyString(), any()))
        .thenAnswer(
            a -> {
              if ("pinLinkValue".equals(a.getArgument(0))) {
                revealPlaceholder = a.getArgument(1);
              }
              if ("pinLinkMode".equals(a.getArgument(0))) {
                revealing = "reveal".equals(a.getArgument(1));
              }
              return forms;
            });
    lenient()
        .when(forms.setError(anyString(), any(Object[].class)))
        .thenAnswer(
            a -> {
              errors.add(a.getArgument(0));
              return forms;
            });
    lenient()
        .when(forms.createForm(anyString()))
        .thenAnswer(
            a ->
                Response.ok(
                        !revealing
                            ? FORM_PAGE
                            : "<html><style id=\"x\"></style>[" + revealPlaceholder + "]</html>")
                    .build());

    session = sessionOn(outer, false);
    provider =
        new PinLinkResourceProvider(
            session,
            () -> publisher,
            () -> MASTER,
            entityManager -> new PinLinkStore("user_attribute", "user_entity"));

    Logger.getLogger("").addHandler(logHandler);
    Logger.getLogger(PinLinkResourceProvider.class.getName()).setLevel(Level.ALL);
  }

  @AfterEach
  void tearDown() throws Exception {
    Logger.getLogger("").removeHandler(logHandler);
    admin.close();
    outer.close();
  }

  @Test
  void isNotFoundWhenThePolicyIsOff() {
    realmAttributes.remove(PinLinkConfig.POLICY);
    assertThat(provider.view(token).getStatus()).isEqualTo(404);
    assertThat(provider.retrieve(token, USERNAME).getStatus()).isEqualTo(404);
  }

  @Test
  void isNotFoundOutsideAnEventRealm() {
    when(realm.getName()).thenReturn("master");
    assertThat(provider.view(token).getStatus()).isEqualTo(404);
  }

  @Test
  void failsClosedWhenThePinIsNotADeclaredSecret() {
    declarePin(Map.of(), Set.of("admin"));
    assertThat(provider.view(token).getStatus()).isEqualTo(404);
    assertThat(provider.retrieve(token, USERNAME).getStatus()).isEqualTo(404);
  }

  @Test
  void failsClosedWhenVotersCanViewThePin() {
    declarePin(Map.of("sequent.secret", "true"), Set.of("admin", "user"));
    assertThat(provider.retrieve(token, USERNAME).getStatus()).isEqualTo(404);
  }

  @Test
  void failsClosedWhenTheGeneratedAttributesAreDeclaredInTheProfile() {
    profile.addOrReplaceAttribute(new UPAttribute(HASH_NAME));
    assertThat(provider.retrieve(token, USERNAME).getStatus()).isEqualTo(404);
  }

  @Test
  void failsClosedWhenVotersCanEditUnmanagedAttributes() {
    profile.setUnmanagedAttributePolicy(UPConfig.UnmanagedAttributePolicy.ENABLED);
    assertThat(provider.retrieve(token, USERNAME).getStatus()).isEqualTo(404);
  }

  @Test
  void getNeverConsumesSoMailScannersCanPrefetch() throws Exception {
    for (int i = 0; i < 5; i++) {
      Response response = provider.view(token);
      assertThat(response.getStatus()).isEqualTo(200);
      assertThat(String.valueOf(response.getEntity())).doesNotContain(PIN);
    }
    assertThat(values(HASH_NAME)).hasSize(1);
    assertThat(values("pin-link-attempts")).isEmpty();
    verify(publisher, never())
        .publishElectoralLogConfirmed(
            any(), any(), any(), any(), any(), any(), org.mockito.ArgumentMatchers.anyLong());
    assertThat(body(provider.retrieve(token, USERNAME))).contains(PIN);
  }

  @Test
  void onlyAPostWithTheRightUsernameConsumesAndTheNextOneIsRefused() throws Exception {
    Response first = provider.retrieve(token, USERNAME);
    assertThat(body(first)).contains(PIN);
    assertThat(values(HASH_NAME)).isEmpty();
    assertThat(values("pin")).hasSize(1);

    Response second = provider.retrieve(token, USERNAME);
    assertThat(String.valueOf(second.getEntity())).doesNotContain(PIN);
  }

  @Test
  void everyRefusalLooksTheSame() throws Exception {
    List<Response> responses = new ArrayList<>();

    responses.add(provider.retrieve("0000000000000000000000000Z", USERNAME)); // unknown
    responses.add(provider.retrieve("garbage", USERNAME)); // malformed
    responses.add(provider.retrieve(token, "someone-else")); // wrong username

    when(user.isEnabled()).thenReturn(false);
    responses.add(provider.retrieve(token, USERNAME)); // disabled voter
    when(user.isEnabled()).thenReturn(true);

    realmAttributes.put(PinLinkConfig.EXPIRES_AT, "2000-01-01T00:00:00Z");
    responses.add(provider.retrieve(token, USERNAME)); // expired
    realmAttributes.put(PinLinkConfig.EXPIRES_AT, "2099-01-01T00:00:00Z");

    responses.add(provider.retrieve(token, "someone-else")); // second wrong username
    responses.add(provider.retrieve(token, USERNAME)); // locked

    resetAttempts();
    when(protector.isTemporarilyDisabled(any(), any(), any())).thenReturn(true);
    responses.add(provider.retrieve(token, USERNAME)); // brute-force lock
    when(protector.isTemporarilyDisabled(any(), any(), any())).thenReturn(false);

    assertThat(body(provider.retrieve(token, USERNAME))).contains(PIN);
    responses.add(provider.retrieve(token, USERNAME)); // used

    assertThat(errors).hasSize(responses.size()).containsOnly("pinLinkGenericError");
    for (Response response : responses) {
      assertThat(response.getStatus()).isEqualTo(200);
      assertThat(stripNonces(String.valueOf(response.getEntity())))
          .isEqualTo(stripNonces(String.valueOf(responses.get(0).getEntity())));
      assertThat(response.getHeaderString("Cache-Control")).isEqualTo("no-store");
    }
  }

  @Test
  void aRevealRecordsAnEntryWithoutTheValueBeforeShowingIt() throws Exception {
    assertThat(body(provider.retrieve(token, USERNAME))).contains(PIN);
    org.mockito.ArgumentCaptor<String> entry = org.mockito.ArgumentCaptor.forClass(String.class);
    verify(publisher)
        .publishElectoralLogConfirmed(
            eq(EVENT),
            eq("VOTER_SECRET_ATTRIBUTE: PIN_LINK_REVEAL"),
            entry.capture(),
            eq(VOTER_ID),
            eq(TENANT),
            eq(USERNAME),
            org.mockito.ArgumentMatchers.anyLong());
    assertThat(entry.getValue())
        .contains("\"pin\"")
        .contains(VOTER_ID)
        .doesNotContain(PIN)
        .doesNotContain(token);
  }

  @Test
  void anUnrecordableRevealShowsNothingAndKeepsTheLink() throws Exception {
    doThrow(new java.io.IOException("broker down"))
        .when(publisher)
        .publishElectoralLogConfirmed(
            any(), any(), any(), any(), any(), any(), org.mockito.ArgumentMatchers.anyLong());

    Response response = provider.retrieve(token, USERNAME);

    assertThat(String.valueOf(response.getEntity())).doesNotContain(PIN);
    assertThat(errors).containsExactly("pinLinkGenericError");
    assertThat(values(HASH_NAME)).hasSize(1);
  }

  @Test
  void aPageThatCannotRenderKeepsTheLink() throws Exception {
    when(forms.createForm(anyString()))
        .thenThrow(new IllegalStateException("broken theme"))
        .thenReturn(Response.ok(FORM_PAGE).build());
    Response response = provider.retrieve(token, USERNAME);
    assertThat(String.valueOf(response.getEntity())).doesNotContain(PIN);
    assertThat(values(HASH_NAME)).hasSize(1);
  }

  @Test
  void theRevealPageIsStrictlyLockedDown() throws Exception {
    Response response = provider.retrieve(token, USERNAME);
    body(response);
    assertStrictHeaders(response);
  }

  @Test
  void theFormIsStrictlyLockedDownToo() {
    assertStrictHeaders(provider.view(token));
  }

  @Test
  void wrongUsernamesFeedKeycloaksBruteForceProtection() {
    provider.retrieve(token, "someone-else");
    verify(protector).failedLogin(eq(realm), eq(user), any(), any(), eq("password"));
  }

  @Test
  void aTemporarilyLockedVoterIsRefusedWithoutSpendingTheLink() throws Exception {
    when(protector.isTemporarilyDisabled(any(), any(), any())).thenReturn(true);
    assertThat(String.valueOf(provider.retrieve(token, USERNAME).getEntity())).doesNotContain(PIN);
    assertThat(values(HASH_NAME)).hasSize(1);
  }

  @Test
  void logsNameTheVoterAndNeverThePinOrToken() throws Exception {
    provider.view(token);
    provider.retrieve(token, "someone-else");
    body(provider.retrieve(token, USERNAME));

    List<String> messages = logs.stream().map(LogRecord::getMessage).toList();
    assertThat(messages).anyMatch(m -> m.contains("opened for voter " + VOTER_ID));
    assertThat(messages)
        .anyMatch(m -> m.contains("refused for voter " + VOTER_ID + ": wrong_username"));
    assertThat(messages).anyMatch(m -> m.contains("revealed for voter " + VOTER_ID));
    assertThat(messages)
        .noneMatch(
            m ->
                m.contains(PIN) || m.contains(token) || m.contains(PinLinkToken.lookupHash(token)));
  }

  @Test
  void splicesThePinEscaped() {
    byte[] page =
        PinLinkResourceProvider.splice(
            "before[X]after", "[X]", "a<b>&\"'".getBytes(StandardCharsets.UTF_8));
    assertThat(new String(page, StandardCharsets.UTF_8))
        .isEqualTo("beforea&lt;b&gt;&amp;&#34;&#39;after");
  }

  private void assertStrictHeaders(Response response) {
    assertThat(response.getHeaderString("Cache-Control")).isEqualTo("no-store");
    assertThat(response.getHeaderString("Referrer-Policy")).isEqualTo("no-referrer");
    assertThat(response.getHeaderString("X-Frame-Options")).isEqualTo("DENY");
    assertThat(response.getHeaderString("X-Content-Type-Options")).isEqualTo("nosniff");
    assertThat(response.getHeaderString("X-Robots-Tag")).isEqualTo("none");
    assertThat(response.getHeaderString("Strict-Transport-Security")).isEqualTo("max-age=31536000");
    String policy = response.getHeaderString("Content-Security-Policy");
    assertThat(policy)
        .startsWith("default-src 'none'")
        .contains("form-action 'self'", "frame-ancestors 'none'", "base-uri 'none'")
        .doesNotContain("unsafe-inline", "unsafe-eval", "*", "http://elsewhere");
  }

  private static String stripNonces(String page) {
    return page.replaceAll("nonce=\"[^\"]*\"", "nonce");
  }

  private String body(Response response) throws Exception {
    assertThat(response.getStatus()).isEqualTo(200);
    if (!(response.getEntity() instanceof StreamingOutput output)) {
      return String.valueOf(response.getEntity());
    }
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    output.write(bytes);
    return bytes.toString(StandardCharsets.UTF_8);
  }

  private void declarePin(Map<String, Object> annotations, Set<String> viewers) {
    UPAttribute pin = new UPAttribute("pin");
    pin.setAnnotations(annotations);
    pin.setPermissions(new UPAttributePermissions(viewers, Set.of("admin")));
    profile.addOrReplaceAttribute(pin);
  }

  private KeycloakSession sessionOn(Connection connection, boolean job) throws Exception {
    KeycloakSession mocked = mock(KeycloakSession.class);
    EntityManager entityManager = mock(EntityManager.class);
    Session hibernate = mock(Session.class);
    JpaConnectionProvider jpa = mock(JpaConnectionProvider.class);
    lenient().when(jpa.getEntityManager()).thenReturn(entityManager);
    lenient().when(entityManager.unwrap(Session.class)).thenReturn(hibernate);
    lenient()
        .when(hibernate.doReturningWork(any()))
        .thenAnswer(a -> a.getArgument(0, ReturningWork.class).execute(connection));
    lenient().when(mocked.getProvider(JpaConnectionProvider.class)).thenReturn(jpa);
    KeycloakContext context = mock(KeycloakContext.class);
    lenient().when(context.getRealm()).thenReturn(realm);
    lenient().when(mocked.getContext()).thenReturn(context);
    RealmProvider realms = mock(RealmProvider.class);
    lenient().when(realms.getRealm(anyString())).thenReturn(realm);
    lenient().when(mocked.realms()).thenReturn(realms);
    TracingProvider tracing = mock(TracingProvider.class, org.mockito.Answers.RETURNS_DEEP_STUBS);
    lenient().when(mocked.getProvider(TracingProvider.class)).thenReturn(tracing);
    KeycloakSessionFactory factory = mock(KeycloakSessionFactory.class);
    lenient().when(factory.create()).thenAnswer(a -> jobSession());
    lenient().when(mocked.getKeycloakSessionFactory()).thenReturn(factory);
    if (job) {
      return mocked;
    }
    lenient().when(context.getUri()).thenReturn(mock(org.keycloak.models.KeycloakUriInfo.class));
    lenient()
        .when(context.getUri().getBaseUri())
        .thenReturn(URI.create("https://login.example.org/"));
    lenient().when(context.getConnection()).thenReturn(mock(ClientConnection.class));
    lenient().when(mocked.getProvider(LoginFormsProvider.class)).thenReturn(forms);
    lenient().when(mocked.getProvider(BruteForceProtector.class)).thenReturn(protector);
    SecurityHeadersProvider headers = mock(SecurityHeadersProvider.class);
    lenient().when(headers.options()).thenReturn(mock(SecurityHeadersOptions.class));
    lenient().when(mocked.getProvider(SecurityHeadersProvider.class)).thenReturn(headers);
    UserProfileProvider userProfile = mock(UserProfileProvider.class);
    lenient().when(userProfile.getConfiguration()).thenReturn(profile);
    lenient().when(mocked.getProvider(UserProfileProvider.class)).thenReturn(userProfile);
    UserProvider users = mock(UserProvider.class);
    lenient().when(users.getUserById(realm, VOTER_ID)).thenReturn(user);
    lenient().when(mocked.users()).thenReturn(users);
    return mocked;
  }

  /** A job session whose transaction manager drives a connection of its own. */
  private KeycloakSession jobSession() throws Exception {
    Connection connection = DriverManager.getConnection(url);
    connection.setAutoCommit(false);
    KeycloakSession job = sessionOn(connection, true);
    boolean[] rollback = {false};
    KeycloakTransactionManager transaction = mock(KeycloakTransactionManager.class);
    lenient().when(transaction.isActive()).thenReturn(true);
    lenient().when(transaction.getRollbackOnly()).thenAnswer(a -> rollback[0]);
    org.mockito.stubbing.Answer<Object> rollbackAnswer =
        a -> {
          rollback[0] = true;
          return null;
        };
    lenient().doAnswer(rollbackAnswer).when(transaction).setRollbackOnly();
    lenient().doAnswer(rollbackAnswer).when(transaction).rollback();
    lenient().when(job.getTransactionManager()).thenReturn(transaction);
    lenient()
        .doAnswer(
            a -> {
              if (!connection.isClosed()) {
                if (rollback[0]) connection.rollback();
                else connection.commit();
                connection.close();
              }
              return null;
            })
        .when(job)
        .close();
    return job;
  }

  private void insertAttribute(String name, String value) throws Exception {
    try (PreparedStatement statement =
        admin.prepareStatement("INSERT INTO user_attribute VALUES (?, ?, ?, ?)")) {
      statement.setString(1, UUID.randomUUID().toString());
      statement.setString(2, VOTER_ID);
      statement.setString(3, name);
      statement.setString(4, value);
      statement.executeUpdate();
    }
  }

  private void resetAttempts() throws Exception {
    try (Statement statement = admin.createStatement()) {
      statement.execute("DELETE FROM user_attribute WHERE name = 'pin-link-attempts'");
    }
  }

  private List<String> values(String name) throws Exception {
    try (PreparedStatement statement =
        admin.prepareStatement("SELECT value FROM user_attribute WHERE name = ?")) {
      statement.setString(1, name);
      try (ResultSet rows = statement.executeQuery()) {
        List<String> values = new ArrayList<>();
        while (rows.next()) {
          values.add(rows.getString(1));
        }
        return values;
      }
    }
  }
}
