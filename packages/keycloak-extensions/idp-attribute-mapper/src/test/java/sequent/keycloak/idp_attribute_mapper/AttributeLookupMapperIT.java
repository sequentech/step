// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_attribute_mapper;

import static java.util.stream.Collectors.joining;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import dasniko.testcontainers.keycloak.KeycloakContainer;
import jakarta.ws.rs.core.Response;
import java.io.File;
import java.io.IOException;
import java.net.CookieHandler;
import java.net.CookieManager;
import java.net.CookiePolicy;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpRequest.BodyPublishers;
import java.net.http.HttpResponse;
import java.net.http.HttpResponse.BodyHandlers;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.stream.Stream;
import org.jsoup.Jsoup;
import org.jsoup.nodes.Document;
import org.jsoup.nodes.Element;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.keycloak.admin.client.Keycloak;
import org.keycloak.admin.client.resource.RealmResource;
import org.keycloak.admin.client.resource.UserProfileResource;
import org.keycloak.models.Constants;
import org.keycloak.representations.idm.CredentialRepresentation;
import org.keycloak.representations.idm.IdentityProviderMapperRepresentation;
import org.keycloak.representations.idm.IdentityProviderRepresentation;
import org.keycloak.representations.idm.RequiredActionProviderRepresentation;
import org.keycloak.representations.idm.UserRepresentation;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.testcontainers.DockerClientFactory;

/**
 * Logs voters in through an OIDC and a SAML identity provider on a Keycloak container that has the
 * mapper and the brokered value condition installed. The {@code idp} realm plays both identity
 * providers; the {@code event} realm has the mappers and the checks from the tutorial.
 */
class AttributeLookupMapperIT {

  private static final String[] PROVIDER_MODULES = {
    "idp-attribute-mapper",
    "conditional-authenticators",
    "message-otp-authenticator",
    "action-token-login-bridge"
  };

  private static final String PASSWORD = "voter-password";
  private static final String PORTAL_CALLBACK = "http://portal.test/callback";
  private static final String AREA_ATTRIBUTE = "area-id";
  private static final String NORTH = "area-north";
  private static final String SOUTH = "area-south";
  private static final String LOOKUP_TABLE =
      String.join(
          "\n",
          "# area = region:precinct",
          NORTH + " = 09:0001..0420, 09:0500",
          SOUTH + " = 09:0421..0499");
  private static final String INVALID_VOTER_ID = "Your voter ID is not valid.";
  private static final String UNKNOWN_PRECINCT = "Your precinct is not part of this election.";

  private static KeycloakContainer keycloak;
  private static Keycloak admin;

  @BeforeAll
  static void startKeycloak() {
    assumeTrue(DockerClientFactory.instance().isDockerAvailable(), "Docker is not available");
    File extensions =
        new File(Objects.requireNonNull(System.getProperty("extensions.dir"), "extensions.dir"));
    List<File> jars =
        Stream.of(PROVIDER_MODULES)
            .map(module -> new File(extensions, module + "/target/sequent." + module + ".jar"))
            .toList();
    keycloak =
        new KeycloakContainer(
                Objects.requireNonNull(System.getProperty("keycloak.image"), "keycloak.image"))
            .withProviderLibsFrom(jars)
            .withRealmImportFiles("/it/idp-realm.json", "/it/event-realm.json");
    keycloak.start();
    admin = keycloak.getKeycloakAdminClient();

    enableUnmanagedAttributes(realm("idp"));
    enableUnmanagedAttributes(realm("event"));
    // SAML voters arrive without names, which would otherwise send them to the profile form.
    RequiredActionProviderRepresentation verifyProfile =
        realm("event").flows().getRequiredAction("VERIFY_PROFILE");
    verifyProfile.setEnabled(false);
    realm("event").flows().updateRequiredAction("VERIFY_PROFILE", verifyProfile);

    addIdentityProviders();
    for (String idp : List.of("oidc", "saml")) {
      String voterIdSource = idp.equals("oidc") ? "voter_id" : MapperSettings.SOURCE_NAME_ID;
      addMapper(
          idp,
          "voter id",
          Map.of(
              MapperSettings.SOURCES,
              voterIdSource,
              MapperSettings.EXTRACT_REGEX,
              "^([^@]+)@",
              MapperSettings.CASE,
              "LOWER",
              MapperSettings.TARGET,
              "USERNAME",
              "syncMode",
              "IMPORT"));
      addMapper(
          idp,
          "area",
          Map.of(
              MapperSettings.SOURCES,
              "region" + Constants.CFG_DELIMITER + "precinct",
              MapperSettings.LOOKUP_TABLE,
              LOOKUP_TABLE,
              MapperSettings.TARGET,
              "USER_ATTRIBUTE",
              MapperSettings.TARGET_ATTRIBUTE,
              AREA_ATTRIBUTE,
              "syncMode",
              "FORCE"));
    }
  }

  @AfterAll
  static void stopKeycloak() {
    if (keycloak != null) {
      keycloak.stop();
    }
  }

  @Test
  void oidcVoterGetsUsernameAndAreaOnFirstLogin() throws Exception {
    createVoter("oidc-valid", "3F2B8C1A-9D4E-8A7B-B1C2-0D9E8F7A6B5C@idp.test", "09", "0150");

    Outcome outcome = new Browser().login("oidc", "oidc-valid");

    assertTrue(outcome.reachedPortal(), outcome.page());
    assertEquals(NORTH, area(eventUser("3f2b8c1a-9d4e-8a7b-b1c2-0d9e8f7a6b5c")));
  }

  @Test
  void samlVoterGetsUsernameFromNameIdAndAreaFromAttributes() throws Exception {
    createVoter("saml-valid", "5e6f7a8b-9c0d-8e1f-a2b3-c4d5e6f7a8b9@idp.test", "09", "0430");

    Outcome outcome = new Browser().login("saml", "saml-valid");

    assertTrue(outcome.reachedPortal(), outcome.page());
    assertEquals(SOUTH, area(eventUser("5e6f7a8b-9c0d-8e1f-a2b3-c4d5e6f7a8b9")));
  }

  @Test
  void invalidVoterIdIsDeniedBeforeTheUserIsCreated() throws Exception {
    createVoter("oidc-not-uuid", "not-a-uuid@idp.test", "09", "0150");
    createVoter("oidc-no-domain", "1c4d2e3f-5a6b-8c7d-9e8f-a0b1c2d3e4f5", "09", "0150");

    Outcome notUuid = new Browser().login("oidc", "oidc-not-uuid");
    Outcome noDomain = new Browser().login("oidc", "oidc-no-domain");

    assertDenied(notUuid, INVALID_VOTER_ID);
    assertDenied(noDomain, INVALID_VOTER_ID);
    assertTrue(eventUsers("not-a-uuid").isEmpty());
    assertTrue(eventUsers("oidc-no-domain").isEmpty());
  }

  @Test
  void unknownPrecinctIsDeniedBeforeTheUserIsCreated() throws Exception {
    createVoter("oidc-unknown", "7a8b9c0d-1e2f-8a3b-8c4d-5e6f7a8b9c0d@idp.test", "09", "0999");

    Outcome outcome = new Browser().login("oidc", "oidc-unknown");

    assertDenied(outcome, UNKNOWN_PRECINCT);
    assertTrue(eventUsers("7a8b9c0d-1e2f-8a3b-8c4d-5e6f7a8b9c0d").isEmpty());
  }

  @Test
  void returningVoterGetsTheNewAreaOrIsDeniedByThePostLoginFlow() throws Exception {
    String username = "0f1e2d3c-4b5a-8968-a776-655443322110";
    createVoter("oidc-mover", username + "@idp.test", "09", "0150");
    assertTrue(new Browser().login("oidc", "oidc-mover").reachedPortal());
    assertEquals(NORTH, area(eventUser(username)));

    setPrecinct("oidc-mover", "0450");
    Outcome moved = new Browser().login("oidc", "oidc-mover");
    assertTrue(moved.reachedPortal(), moved.page());
    assertEquals(SOUTH, area(eventUser(username)));

    setPrecinct("oidc-mover", "0999");
    Outcome left = new Browser().login("oidc", "oidc-mover");
    assertDenied(left, UNKNOWN_PRECINCT);
    // The mapper updates the user before the post login flow runs.
    assertNull(area(eventUser(username)));
  }

  private static RealmResource realm(String name) {
    return admin.realm(name);
  }

  private static void enableUnmanagedAttributes(RealmResource realm) {
    UserProfileResource profile = realm.users().userProfile();
    UPConfig config = profile.getConfiguration();
    config.setUnmanagedAttributePolicy(UPConfig.UnmanagedAttributePolicy.ENABLED);
    profile.update(config);
  }

  /**
   * Keycloak's address as the browser sees it. Java's cookie manager never returns cookies for a
   * host without a dot, such as {@code localhost}.
   */
  private static String browserUrl() {
    String host = "localhost".equals(keycloak.getHost()) ? "127.0.0.1" : keycloak.getHost();
    return "http://" + host + ":" + keycloak.getHttpPort();
  }

  private static void addIdentityProviders() {
    // The browser reaches the idp realm through the mapped port; Keycloak calls itself on 8080.
    String browserBase = browserUrl() + "/realms/idp/protocol";
    String containerBase = "http://localhost:8080/realms/idp/protocol";
    createIdentityProvider(
        "oidc",
        "keycloak-oidc",
        Map.of(
            "clientId", "event-oidc-broker",
            "clientSecret", "event-oidc-broker-secret",
            "clientAuthMethod", "client_secret_post",
            "authorizationUrl", browserBase + "/openid-connect/auth",
            "tokenUrl", containerBase + "/openid-connect/token",
            "defaultScope", "openid",
            "validateSignature", "false",
            "disableUserInfo", "true",
            "syncMode", "IMPORT"));
    createIdentityProvider(
        "saml",
        "saml",
        Map.of(
            "singleSignOnServiceUrl", browserBase + "/saml",
            "entityId", "event-saml-sp",
            "nameIDPolicyFormat", "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress",
            "principalType", "SUBJECT",
            "postBindingResponse", "true",
            "postBindingAuthnRequest", "false",
            "validateSignature", "false",
            "wantAuthnRequestsSigned", "false",
            "syncMode", "IMPORT"));
  }

  private static void createIdentityProvider(
      String alias, String providerId, Map<String, String> config) {
    IdentityProviderRepresentation idp = new IdentityProviderRepresentation();
    idp.setAlias(alias);
    idp.setProviderId(providerId);
    idp.setEnabled(true);
    idp.setFirstBrokerLoginFlowAlias("voter first broker login");
    idp.setPostBrokerLoginFlowAlias("voter post login");
    idp.setConfig(new HashMap<>(config));
    try (Response response = realm("event").identityProviders().create(idp)) {
      assertEquals(201, response.getStatus(), "create identity provider " + alias);
    }
  }

  private static void addMapper(String idpAlias, String name, Map<String, String> config) {
    IdentityProviderMapperRepresentation mapper = new IdentityProviderMapperRepresentation();
    mapper.setName(name);
    mapper.setIdentityProviderAlias(idpAlias);
    mapper.setIdentityProviderMapper(AttributeLookupMapper.PROVIDER_ID);
    mapper.setConfig(new HashMap<>(config));
    try (Response response = realm("event").identityProviders().get(idpAlias).addMapper(mapper)) {
      assertEquals(201, response.getStatus(), "create mapper " + idpAlias + "/" + name);
    }
  }

  private static void createVoter(String username, String voterId, String region, String precinct) {
    CredentialRepresentation password = new CredentialRepresentation();
    password.setType(CredentialRepresentation.PASSWORD);
    password.setValue(PASSWORD);
    password.setTemporary(false);
    UserRepresentation user = new UserRepresentation();
    user.setUsername(username);
    user.setEnabled(true);
    user.setEmail(voterId.contains("@") ? voterId : voterId + "@nameless.test");
    user.setEmailVerified(true);
    user.setFirstName("Test");
    user.setLastName("Voter");
    user.setAttributes(
        Map.of(
            "voter_id",
            List.of(voterId),
            "region",
            List.of(region),
            "precinct",
            List.of(precinct)));
    user.setCredentials(List.of(password));
    try (Response response = realm("idp").users().create(user)) {
      assertEquals(201, response.getStatus(), "create voter " + username);
    }
  }

  private static void setPrecinct(String idpUsername, String precinct) {
    UserRepresentation found = realm("idp").users().search(idpUsername, true).get(0);
    UserRepresentation user = realm("idp").users().get(found.getId()).toRepresentation();
    Map<String, List<String>> attributes = new HashMap<>(user.getAttributes());
    attributes.put("precinct", List.of(precinct));
    user.setAttributes(attributes);
    realm("idp").users().get(user.getId()).update(user);
  }

  private static List<UserRepresentation> eventUsers(String username) {
    return realm("event").users().search(username, true);
  }

  private static UserRepresentation eventUser(String username) {
    List<UserRepresentation> found = eventUsers(username);
    assertEquals(1, found.size(), "event realm user " + username);
    return realm("event").users().get(found.get(0).getId()).toRepresentation();
  }

  private static String area(UserRepresentation user) {
    List<String> values =
        user.getAttributes() == null ? null : user.getAttributes().get(AREA_ATTRIBUTE);
    return values == null || values.isEmpty() ? null : values.get(0);
  }

  private static void assertDenied(Outcome outcome, String message) {
    assertFalse(outcome.reachedPortal(), "login should be denied");
    assertTrue(outcome.page().contains(message), outcome.page());
  }

  /** Where a login ended: the portal callback, or the text of the last page Keycloak showed. */
  private record Outcome(URI portal, String page) {
    boolean reachedPortal() {
      return portal != null;
    }
  }

  /** Keeps cookies, follows redirects, fills the IdP login form and posts SAML forms. */
  private static final class Browser {
    private static final int MAX_STEPS = 30;

    private final HttpClient http =
        HttpClient.newBuilder()
            .cookieHandler(new HttpCookieHandler())
            .followRedirects(HttpClient.Redirect.NEVER)
            .build();

    Outcome login(String idpAlias, String idpUsername) throws Exception {
      URI start =
          URI.create(
              browserUrl()
                  + "/realms/event/protocol/openid-connect/auth?client_id=portal"
                  + "&response_type=code&scope=openid&redirect_uri="
                  + encode(PORTAL_CALLBACK)
                  + "&kc_idp_hint="
                  + idpAlias);
      HttpRequest request = HttpRequest.newBuilder(start).GET().build();
      boolean credentialsSent = false;
      for (int step = 0; step < MAX_STEPS; step++) {
        HttpResponse<String> response = http.send(request, BodyHandlers.ofString());
        Optional<String> location = response.headers().firstValue("Location");
        if (response.statusCode() / 100 == 3 && location.isPresent()) {
          URI next = request.uri().resolve(location.get());
          if (next.toString().startsWith(PORTAL_CALLBACK)) {
            return new Outcome(next, null);
          }
          request = HttpRequest.newBuilder(next).GET().build();
          continue;
        }
        Document page = Jsoup.parse(response.body(), request.uri().toString());
        Element samlForm = page.selectFirst("form:has(input[name=SAMLResponse])");
        Element loginForm = page.selectFirst("form#kc-form-login");
        if (samlForm != null) {
          request = post(samlForm, Map.of());
        } else if (loginForm != null && !credentialsSent) {
          credentialsSent = true;
          request = post(loginForm, Map.of("username", idpUsername, "password", PASSWORD));
        } else {
          return new Outcome(null, page.text());
        }
      }
      throw new AssertionError("The login did not finish in " + MAX_STEPS + " steps");
    }

    private static HttpRequest post(Element form, Map<String, String> fields) {
      Map<String, String> data = new LinkedHashMap<>();
      for (Element input : form.select("input[name]")) {
        data.put(input.attr("name"), input.attr("value"));
      }
      data.putAll(fields);
      String body =
          data.entrySet().stream()
              .map(field -> encode(field.getKey()) + "=" + encode(field.getValue()))
              .collect(joining("&"));
      return HttpRequest.newBuilder(URI.create(form.absUrl("action")))
          .header("Content-Type", "application/x-www-form-urlencoded")
          .POST(BodyPublishers.ofString(body))
          .build();
    }

    private static String encode(String value) {
      return URLEncoder.encode(value, StandardCharsets.UTF_8);
    }
  }

  /**
   * Keycloak marks its cookies Secure even over HTTP, which browsers accept on a local address but
   * Java's cookie manager doesn't. This one stores them without the flag.
   */
  private static final class HttpCookieHandler extends CookieHandler {
    private final CookieManager cookies = new CookieManager(null, CookiePolicy.ACCEPT_ALL);

    @Override
    public Map<String, List<String>> get(URI uri, Map<String, List<String>> requestHeaders)
        throws IOException {
      return cookies.get(uri, requestHeaders);
    }

    @Override
    public void put(URI uri, Map<String, List<String>> responseHeaders) throws IOException {
      Map<String, List<String>> headers = new HashMap<>();
      responseHeaders.forEach(
          (name, values) ->
              headers.put(
                  name,
                  "Set-Cookie".equalsIgnoreCase(name)
                      ? values.stream().map(v -> v.replaceAll("(?i);\\s*Secure", "")).toList()
                      : values));
      cookies.put(uri, headers);
    }
  }
}
