// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import static org.assertj.core.api.Assertions.assertThat;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.atLeastOnce;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.lenient;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.persistence.EntityManager;
import jakarta.ws.rs.core.MultivaluedHashMap;
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
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
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
import org.keycloak.locale.LocaleSelectorProvider;
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
  private final MultivaluedHashMap<String, String> queryParameters = new MultivaluedHashMap<>();
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

  /**
   * An in-memory database holding one voter, with an encrypted PIN and the hash of its link, and
   * Keycloak's services mocked around the provider.
   */
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

  /**
   * Mail scanners follow links, so any number of GETs shows no PIN, spends no attempt and records
   * no audit entry, and the voter's POST still works afterwards.
   */
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

  /**
   * Every reason to refuse gets the same page with the generic error, so a response doesn't tell
   * the reason: an unknown or malformed link, a wrong username, a disabled voter, an expired or
   * locked link, a brute-force lock and a link already used.
   */
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

  /** A realm's CSS may load a logo from its web site; nothing else in the policy moves for it. */
  @Test
  void aRealmCanAllowImagesAndFontsFromOtherHosts() throws Exception {
    realmAttributes.put(
        "pin-link-asset-origins", "https://assets.example.org, https://cdn.example.org:8443/");
    Response form = provider.view(token);
    Response reveal = provider.retrieve(token, USERNAME);
    for (Response response : List.of(form, reveal)) {
      Map<String, String> policy = directives(response);
      assertThat(policy.get("img-src"))
          .endsWith(" data: https://assets.example.org https://cdn.example.org:8443");
      assertThat(policy.get("font-src"))
          .endsWith(" https://assets.example.org https://cdn.example.org:8443");
      for (String directive :
          List.of(
              "default-src",
              "script-src",
              "style-src",
              "form-action",
              "base-uri",
              "frame-ancestors")) {
        assertThat(policy.get(directive))
            .as(directive)
            .doesNotContain("assets.example.org", "cdn.example.org");
      }
    }
    assertThat(body(reveal)).contains(PIN);
  }

  @Test
  void onlyHttpsOriginsCanBeAllowed() {
    String strict = withoutNonce(provider.view(token));
    for (String value :
        List.of(
            "*",
            "https:",
            "'self'",
            "'unsafe-inline'",
            "data:",
            "http://assets.example.org",
            "https://*.example.org",
            "https://assets.example.org/logo.png",
            "https://user@assets.example.org",
            "https://assets.example.org; script-src *",
            "https://assets.example.org;script-src",
            "javascript:alert(1)")) {
      realmAttributes.put("pin-link-asset-origins", value);
      Response response = provider.view(token);
      assertThat(response.getStatus()).as(value).isEqualTo(200);
      assertThat(withoutNonce(response)).as(value).isEqualTo(strict);
    }
  }

  /** A typo blocks a logo: it does not take the link away from the voters who were mailed it. */
  @Test
  void aBadEntryLeavesTheRestAndTheLink() throws Exception {
    realmAttributes.put(
        "pin-link-asset-origins", "* https://assets.example.org\r\nX-Injected: yes");
    Response response = provider.view(token);
    assertThat(directives(response).get("img-src")).endsWith(" data: https://assets.example.org");
    assertThat(withoutNonce(response)).doesNotContain("X-Injected", "yes", "*", "\r", "\n");
    assertThat(body(provider.retrieve(token, USERNAME))).contains(PIN);
  }

  /** The theme's own stylesheet is inline, written with or without attributes. */
  @Test
  void everyInlineStyleRunsUnderTheNonce() {
    when(forms.createForm(anyString()))
        .thenReturn(
            Response.ok("<style>a{}</style><style id=\"x\">b{}</style><stylesheet>c</stylesheet>")
                .build());
    Response response = provider.view(token);
    Matcher nonce =
        Pattern.compile("'nonce-([^']+)'")
            .matcher(response.getHeaderString("Content-Security-Policy"));
    assertThat(nonce.find()).isTrue();
    String expected = "<style nonce=\"" + nonce.group(1) + "\"";
    assertThat(String.valueOf(response.getEntity()))
        .isEqualTo(
            expected
                + ">a{}</style>"
                + expected
                + " id=\"x\">b{}</style><stylesheet>c</stylesheet>");
  }

  /** A short host serves this page by the token alone, not by Keycloak's path. */
  @Test
  void theLanguageMenuStaysOnTheHostTheVoterCameThrough() {
    when(forms.createForm(anyString()))
        .thenReturn(
            Response.ok(
                    "<a href=\"/realms/r/pin-link/T?kc_locale=es\">es</a>"
                        + "<a href=\"https://login.example.org/realms/r/pin-link/T?kc_locale=ca\">ca</a>"
                        + "<a href=\"/realms/r/account\">other</a>")
                .build());
    assertThat(String.valueOf(provider.view(token).getEntity()))
        .isEqualTo(
            "<a href=\"?kc_locale=es\">es</a><a href=\"?kc_locale=ca\">ca</a>"
                + "<a href=\"/realms/r/account\">other</a>");
  }

  /** The theme's language menu links to this page with kc_locale, and no login flow reads it. */
  @Test
  void theLanguagePickedInTheMenuReachesTheLocaleSelector() {
    queryParameters.putSingle(LocaleSelectorProvider.KC_LOCALE_PARAM, "es");
    provider.view(token);
    verify(session).setAttribute(LocaleSelectorProvider.USER_REQUEST_LOCALE, "es");
  }

  @Test
  void theRevealKeepsTheLanguagePicked() throws Exception {
    queryParameters.putSingle(LocaleSelectorProvider.KC_LOCALE_PARAM, "es");
    body(provider.retrieve(token, USERNAME));
    verify(session).setAttribute(LocaleSelectorProvider.USER_REQUEST_LOCALE, "es");
  }

  @Test
  void aRefusalKeepsTheLanguagePicked() {
    queryParameters.putSingle(LocaleSelectorProvider.KC_LOCALE_PARAM, "es");
    provider.retrieve(token, "someone-else");
    verify(session, atLeastOnce()).setAttribute(LocaleSelectorProvider.USER_REQUEST_LOCALE, "es");
  }

  @Test
  void withoutAPickTheLocaleSelectorDecides() {
    provider.view(token);
    verify(session, never()).setAttribute(eq(LocaleSelectorProvider.USER_REQUEST_LOCALE), any());
  }

  @Test
  void pickingALanguageDoesNotSpendTheLink() throws Exception {
    queryParameters.putSingle(LocaleSelectorProvider.KC_LOCALE_PARAM, "es");
    assertThat(provider.view(token).getStatus()).isEqualTo(200);
    assertThat(values(HASH_NAME)).hasSize(1);
    assertThat(String.valueOf(provider.view(token).getEntity())).doesNotContain(PIN);
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

  /**
   * A deployment mistake and not the voter's: it has to stand out in the log, without the value,
   * and it can't cost the voter the link.
   */
  @Test
  void aMissingMasterSecretIsAnErrorInTheLogAndKeepsTheLink() throws Exception {
    for (String secret : Arrays.asList(null, "", "not-the-right-format")) {
      logs.clear();
      PinLinkResourceProvider unconfigured =
          new PinLinkResourceProvider(
              session,
              () -> publisher,
              () -> secret,
              entityManager -> new PinLinkStore("user_attribute", "user_entity"));
      assertThat(body(unconfigured.retrieve(token, USERNAME))).doesNotContain(PIN);
      assertThat(values(HASH_NAME)).hasSize(1);
      List<String> errors =
          logs.stream()
              .filter(record -> record.getLevel().intValue() >= Level.SEVERE.intValue())
              .map(LogRecord::getMessage)
              .toList();
      assertThat(errors).as(secret).hasSize(1);
      assertThat(errors.get(0)).contains("MASTER_SECRET").doesNotContain("not-the-right-format");
    }
  }

  /** The logs follow a link by voter id and never hold the PIN, the token or its lookup hash. */
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

  private static Map<String, String> directives(Response response) {
    Map<String, String> policy = new HashMap<>();
    for (String directive : response.getHeaderString("Content-Security-Policy").split("; ")) {
      String[] parts = directive.split(" ", 2);
      policy.put(parts[0], parts[1]);
    }
    return policy;
  }

  private static String withoutNonce(Response response) {
    return response.getHeaderString("Content-Security-Policy").replaceAll("nonce-[^']+", "nonce");
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

  /** The body of a successful response as text, reading a streamed reveal to the end. */
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

  /**
   * A mocked Keycloak session whose database work runs on {@code connection}. A job session, like
   * the one Keycloak opens for a transaction of its own, stops at the database; a request session
   * also has the forms, brute-force protector, user profile and users the provider reads.
   */
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
    lenient().when(context.getUri().getQueryParameters()).thenReturn(queryParameters);
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

  /** The values stored under an attribute name, read straight from the database. */
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
