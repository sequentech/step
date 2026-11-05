// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import static java.util.stream.Collectors.joining;

import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.persistence.EntityManager;
import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.FormParam;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.StreamingOutput;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.security.SecureRandom;
import java.time.Instant;
import java.util.Arrays;
import java.util.Base64;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import java.util.function.Supplier;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import lombok.extern.jbosslog.JBossLog;
import org.hibernate.Session;
import org.keycloak.connections.jpa.JpaConnectionProvider;
import org.keycloak.connections.jpa.util.JpaUtils;
import org.keycloak.events.Details;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.locale.LocaleSelectorProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.PasswordCredentialModel;
import org.keycloak.models.utils.KeycloakModelUtils;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.services.managers.BruteForceProtector;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.userprofile.UserProfileProvider;
import sequent.keycloak.custom_event_listener.RabbitMqEventPublisher;
import sequent.keycloak.realm.RealmNames;

/**
 * Serves {@code /realms/{realm}/pin-link/{token}}: a public page where a voter trades a one-time
 * link and their username for their PIN. Off unless the realm's {@code pin-link-policy} says
 * otherwise.
 *
 * <p>GET only shows the form, because mail scanners follow links. Every refusal shows the same
 * message; the real reason goes to the Keycloak event of the voter the token belongs to. The token
 * and PIN are never logged.
 */
@JBossLog
public class PinLinkResourceProvider implements RealmResourceProvider {
  private static final String TEMPLATE = "pin-link.ftl";
  private static final String GENERIC_ERROR = "pinLinkGenericError";
  private static final String CUSTOM_ACTION = "pin-link";
  private static final String STAGE_DETAIL = "pin_link_stage";
  private static final String DENIED_ERROR = "pin_link_denied";
  private static final String REVEAL_MESSAGE_TYPE = "VOTER_SECRET_ATTRIBUTE: PIN_LINK_REVEAL";
  private static final long AUDIT_CONFIRM_MILLIS = 5_000;
  private static final SecureRandom RANDOM = new SecureRandom();
  private static final ObjectMapper JSON = new ObjectMapper();
  private static final Pattern STYLE_TAG =
      Pattern.compile("<style(?=[\\s>])", Pattern.CASE_INSENSITIVE);
  private static final Pattern LANGUAGE_LINK =
      Pattern.compile("href=\"[^\"]*\\?" + LocaleSelectorProvider.KC_LOCALE_PARAM + "=");

  private final KeycloakSession session;
  private final Supplier<RabbitMqEventPublisher> publisher;
  private final Supplier<String> masterSecret;
  private final Function<EntityManager, PinLinkStore> stores;

  PinLinkResourceProvider(
      KeycloakSession session,
      Supplier<RabbitMqEventPublisher> publisher,
      Supplier<String> masterSecret) {
    this(
        session,
        publisher,
        masterSecret,
        entityManager ->
            new PinLinkStore(
                JpaUtils.getTableNameForNativeQuery("USER_ATTRIBUTE", entityManager),
                JpaUtils.getTableNameForNativeQuery("USER_ENTITY", entityManager)));
  }

  PinLinkResourceProvider(
      KeycloakSession session,
      Supplier<RabbitMqEventPublisher> publisher,
      Supplier<String> masterSecret,
      Function<EntityManager, PinLinkStore> stores) {
    this.session = session;
    this.publisher = publisher;
    this.masterSecret = masterSecret;
    this.stores = stores;
  }

  @Override
  public Object getResource() {
    return this;
  }

  @Override
  public void close() {}

  /**
   * Shows the form for a link and nothing else, because mail scanners follow links. The voter the
   * token belongs to gets an "opened" event, but nothing is written and no attempt is spent. A
   * realm with the link off answers 404.
   */
  @GET
  @Path("{token}")
  public Response view(@PathParam("token") String rawToken) {
    RealmModel realm = session.getContext().getRealm();
    Optional<PinLinkConfig> config = activeConfig(realm);
    if (config.isEmpty()) {
      return Response.status(Response.Status.NOT_FOUND).build();
    }
    // Logged for the voter the link belongs to, and nothing else: no write, no attempt spent.
    PinLinkToken.canonicalize(rawToken)
        .flatMap(token -> findVoter(realm, config.get(), token))
        .ifPresent(voter -> voterEvent(realm, voter, "opened", null));
    return renderForm(false);
  }

  /**
   * The POST that trades the link and the voter's username for their PIN. The link is spent and the
   * reveal audited in one transaction, and the page is rendered before it so a broken theme can't
   * cost the voter their link. Every refusal shows the same form, with the real reason only in the
   * voter's event.
   */
  @POST
  @Path("{token}")
  @Consumes(MediaType.APPLICATION_FORM_URLENCODED)
  public Response retrieve(
      @PathParam("token") String rawToken, @FormParam("username") String username) {
    RealmModel realm = session.getContext().getRealm();
    Optional<PinLinkConfig> found = activeConfig(realm);
    if (found.isEmpty()) {
      return Response.status(Response.Status.NOT_FOUND).build();
    }
    PinLinkConfig config = found.get();
    Optional<String> token = PinLinkToken.canonicalize(rawToken);
    Optional<UserModel> voter = token.flatMap(value -> findVoter(realm, config, value));
    if (voter.isEmpty()) {
      return renderForm(true);
    }
    UserModel user = voter.get();
    if (config.isExpired(Instant.now())) {
      return refuse(realm, user, "expired");
    }
    if (!user.isEnabled()) {
      return refuse(realm, user, "disabled_voter");
    }
    if (isBruteForceLocked(realm, user)) {
      return refuse(realm, user, "brute_force_locked");
    }
    if (username == null || username.length() > 255) {
      return refuse(realm, user, "wrong_username");
    }
    String master = masterSecret.get();
    if (master == null || !master.matches("[0-9a-fA-F]{64}")) {
      log.error("pin-link: MASTER_SECRET is missing or is not 64 hexadecimal characters");
      return refuse(realm, user, "not_configured");
    }
    var scope = RealmNames.parseEventRealmName(realm.getName());
    if (scope.isEmpty()) {
      return refuse(realm, user, "not_configured");
    }

    // Everything that can fail short of the commit happens before it, so a broken theme costs the
    // voter a retry and not their link.
    String placeholder = randomToken();
    String nonce = randomToken();
    String page;
    try {
      page = renderReveal(placeholder, nonce);
    } catch (RuntimeException error) {
      log.errorf("pin-link: could not render the page for voter %s", user.getId());
      return refuse(realm, user, "render_failed");
    }

    PinLinkService.Voter subject =
        new PinLinkService.Voter(
            scope.get().tenantId(),
            scope.get().electionEventId(),
            user.getId(),
            user.getUsername());
    PinLinkService.Outcome outcome;
    try {
      outcome =
          KeycloakModelUtils.runJobInTransactionWithResult(
              session.getKeycloakSessionFactory(),
              job ->
                  withConnection(
                      job,
                      (store, connection) ->
                          new PinLinkService(store, config, master)
                              .retrieve(
                                  connection,
                                  subject,
                                  token.get(),
                                  username,
                                  () -> recordReveal(subject, config))));
    } catch (PinLinkService.AuditFailedException error) {
      log.errorf("pin-link: reveal not recorded, refusing voter %s", user.getId());
      return refuse(realm, user, "audit_failed");
    } catch (RuntimeException error) {
      // By class only: driver messages can quote bound values.
      log.errorf(
          "pin-link: transaction failed for voter %s: %s",
          user.getId(), error.getClass().getName());
      return refuse(realm, user, "transaction_failed");
    }

    if (outcome.reason() != PinLinkService.Reason.REVEALED) {
      if (outcome.reason() == PinLinkService.Reason.WRONG_USERNAME) {
        applyBruteForce(realm, user);
      }
      return refuse(realm, user, outcome.reason().name().toLowerCase(java.util.Locale.ROOT));
    }
    // The commit is behind us: the link is spent, and the entry is on its way to the log.
    voterEvent(realm, user, "revealed", null);
    return revealResponse(page, placeholder, nonce, outcome.pin());
  }

  private Response refuse(RealmModel realm, UserModel user, String reason) {
    voterEvent(realm, user, "refused", reason);
    return renderForm(true);
  }

  /** The config, if this is an event realm that turned the link on and kept the PIN private. */
  private Optional<PinLinkConfig> activeConfig(RealmModel realm) {
    if (RealmNames.parseEventRealmName(realm.getName()).isEmpty()) {
      return Optional.empty();
    }
    return PinLinkConfig.from(realm::getAttribute).filter(this::pinIsSecretAndAdminOnly);
  }

  /**
   * Whether the user profile keeps the PIN private: a declared secret that voters cannot view,
   * while the token hash and attempts attributes stay undeclared and unmanaged attributes are not
   * enabled. Logs the condition that fails.
   */
  private boolean pinIsSecretAndAdminOnly(PinLinkConfig config) {
    UPConfig profile = session.getProvider(UserProfileProvider.class).getConfiguration();
    UPAttribute pin = profile.getAttribute(config.pinAttribute());
    if (pin == null
        || pin.getAnnotations() == null
        || !"true".equalsIgnoreCase(String.valueOf(pin.getAnnotations().get("sequent.secret")))) {
      log.warnf("pin-link: attribute %s is not a declared secret", config.pinAttribute());
      return false;
    }
    if (pin.getPermissions() != null
        && pin.getPermissions().getView() != null
        && pin.getPermissions().getView().contains("user")) {
      log.warnf("pin-link: voters can view attribute %s", config.pinAttribute());
      return false;
    }
    // A declared attribute is one the account console could offer; these two must not be.
    if (profile.getAttribute(config.tokenHashAttribute()) != null
        || profile.getAttribute(config.attemptsAttribute()) != null
        || profile.getUnmanagedAttributePolicy() == UPConfig.UnmanagedAttributePolicy.ENABLED) {
      log.warnf(
          "pin-link: attributes %s and %s must stay undeclared and unmanaged",
          config.tokenHashAttribute(), config.attemptsAttribute());
      return false;
    }
    return true;
  }

  /**
   * The voter the token belongs to, found by its lookup hash in this realm. Empty when no voter has
   * it, several do, or the lookup fails.
   */
  private Optional<UserModel> findVoter(RealmModel realm, PinLinkConfig config, String token) {
    try {
      String hash = PinLinkToken.lookupHash(token);
      Optional<String> voterId =
          withConnection(
              session,
              (store, connection) ->
                  store.findVoterId(connection, realm.getId(), config.tokenHashAttribute(), hash));
      return voterId.map(id -> session.users().getUserById(realm, id));
    } catch (RuntimeException error) {
      log.errorf("pin-link: lookup failed: %s", error.getClass().getName());
      return Optional.empty();
    }
  }

  private interface StoreWork<T> {
    T run(PinLinkStore store, java.sql.Connection connection) throws java.sql.SQLException;
  }

  private <T> T withConnection(KeycloakSession session, StoreWork<T> work) {
    EntityManager entityManager =
        session.getProvider(JpaConnectionProvider.class).getEntityManager();
    PinLinkStore store = stores.apply(entityManager);
    return entityManager
        .unwrap(Session.class)
        .doReturningWork(connection -> work.run(store, connection));
  }

  /** The reveal's electoral-log entry; it names the attribute and voter, never the value. */
  private void recordReveal(PinLinkService.Voter voter, PinLinkConfig config) throws Exception {
    Map<String, Object> voterEntry = new LinkedHashMap<>();
    voterEntry.put("user_id", voter.id());
    voterEntry.put("username", voter.username());
    Map<String, Object> entry = new LinkedHashMap<>();
    entry.put("action", "pin_link_reveal");
    entry.put("attribute_names", List.of(config.pinAttribute()));
    entry.put("voter", voterEntry);
    publisher
        .get()
        .publishElectoralLogConfirmed(
            voter.electionEventId(),
            REVEAL_MESSAGE_TYPE,
            JSON.writeValueAsString(entry),
            voter.id(),
            voter.tenantId(),
            voter.username(),
            AUDIT_CONFIRM_MILLIS);
  }

  /** Whether brute-force protection has the voter locked out, permanently or temporarily. */
  private boolean isBruteForceLocked(RealmModel realm, UserModel user) {
    if (!realm.isBruteForceProtected()) {
      return false;
    }
    BruteForceProtector protector = session.getProvider(BruteForceProtector.class);
    return protector.isPermanentlyLockedOut(session, realm, user)
        || protector.isTemporarilyDisabled(session, realm, user);
  }

  /**
   * Reports a wrong username to Keycloak's brute-force protection as a failed password login: the
   * PIN is the password, and the protector ignores other kinds of failure.
   */
  private void applyBruteForce(RealmModel realm, UserModel user) {
    if (realm.isBruteForceProtected()) {
      // The protector silently drops a failure of any other category, and this PIN is the password.
      session
          .getProvider(BruteForceProtector.class)
          .failedLogin(
              realm,
              user,
              session.getContext().getConnection(),
              session.getContext().getUri(),
              PasswordCredentialModel.TYPE);
    }
  }

  /**
   * Records a stage of the link (opened, refused or revealed) as a Keycloak event of the voter. A
   * reason makes it an error event carrying that reason. It never holds the token or the PIN.
   */
  private void voterEvent(RealmModel realm, UserModel user, String stage, String reason) {
    EventBuilder event =
        new EventBuilder(realm, session, session.getContext().getConnection())
            .event(EventType.CUSTOM_REQUIRED_ACTION)
            .user(user)
            .detail(Details.USERNAME, user.getUsername())
            .detail(Details.CUSTOM_REQUIRED_ACTION, CUSTOM_ACTION)
            .detail(STAGE_DETAIL, stage);
    if (reason == null) {
      log.infof("pin-link: %s for voter %s", stage, user.getId());
      event.success();
    } else {
      log.infof("pin-link: %s for voter %s: %s", stage, user.getId(), reason);
      event.detail(Details.REASON, reason).error(DENIED_ERROR);
    }
  }

  /**
   * The forms provider for this page, in the language picked in the theme's menu. The menu links
   * here with {@code kc_locale}, which Keycloak only reads in a login flow, and this page has none.
   * The locale selector ignores a value the realm does not support.
   */
  private LoginFormsProvider pageForms() {
    String picked =
        session
            .getContext()
            .getUri()
            .getQueryParameters()
            .getFirst(LocaleSelectorProvider.KC_LOCALE_PARAM);
    if (picked != null) {
      session.setAttribute(LocaleSelectorProvider.USER_REQUEST_LOCALE, picked);
    }
    return session.getProvider(LoginFormsProvider.class);
  }

  /**
   * Renders the username form, with the generic error when {@code failed}. It has the same strict
   * headers as the reveal page.
   */
  private Response renderForm(boolean failed) {
    String nonce = randomToken();
    LoginFormsProvider forms = pageForms().setAttribute("pinLinkMode", "form");
    if (failed) {
      forms.setError(GENERIC_ERROR);
    }
    String page = (String) forms.createForm(TEMPLATE).getEntity();
    return secure(Response.ok(withLocalLanguageLinks(withStyleNonce(page, nonce))), nonce).build();
  }

  /**
   * Renders the page that will show the PIN, with {@code placeholder} where it is spliced in. Fails
   * if the theme leaves no place for it, which the caller finds out before it spends the link.
   */
  private String renderReveal(String placeholder, String nonce) {
    String page =
        (String)
            pageForms()
                .setAttribute("pinLinkMode", "reveal")
                .setAttribute("pinLinkValue", placeholder)
                .setAttribute("pinLinkNonce", nonce)
                .createForm(TEMPLATE)
                .getEntity();
    if (!page.contains(placeholder)) {
      throw new IllegalStateException("The theme's page has no place for the PIN");
    }
    return withStyleNonce(page, nonce);
  }

  /** Splices the PIN in as bytes: it never becomes a String, so it can be wiped once written. */
  private Response revealResponse(String page, String placeholder, String nonce, byte[] pin) {
    byte[] body = splice(page, placeholder, pin);
    Arrays.fill(pin, (byte) 0);
    StreamingOutput output =
        stream -> {
          try {
            stream.write(body);
          } finally {
            Arrays.fill(body, (byte) 0);
          }
        };
    return secure(Response.ok(output), nonce).build();
  }

  static byte[] splice(String page, String placeholder, byte[] pin) {
    int at = page.indexOf(placeholder);
    byte[] head = page.substring(0, at).getBytes(StandardCharsets.UTF_8);
    byte[] tail = page.substring(at + placeholder.length()).getBytes(StandardCharsets.UTF_8);
    byte[] escaped = htmlEscape(pin);
    byte[] body = new byte[head.length + escaped.length + tail.length];
    System.arraycopy(head, 0, body, 0, head.length);
    System.arraycopy(escaped, 0, body, head.length, escaped.length);
    System.arraycopy(tail, 0, body, head.length + escaped.length, tail.length);
    Arrays.fill(escaped, (byte) 0);
    return body;
  }

  /**
   * Escapes the characters HTML treats specially. It works on bytes so the PIN never becomes a
   * String, and wipes its scratch buffer.
   */
  private static byte[] htmlEscape(byte[] value) {
    byte[] out = new byte[value.length * 6];
    int size = 0;
    for (byte b : value) {
      String entity =
          switch (b) {
            case '&' -> "&amp;";
            case '<' -> "&lt;";
            case '>' -> "&gt;";
            case '"' -> "&#34;";
            case '\'' -> "&#39;";
            default -> null;
          };
      if (entity == null) {
        out[size++] = b;
      } else {
        for (byte e : entity.getBytes(StandardCharsets.US_ASCII)) {
          out[size++] = e;
        }
      }
    }
    byte[] escaped = Arrays.copyOf(out, size);
    Arrays.fill(out, (byte) 0);
    return escaped;
  }

  /**
   * The theme's language menu links to this page by Keycloak's path, which a short host does not
   * serve. A link to the query alone works on whichever host the voter came through.
   */
  private static String withLocalLanguageLinks(String page) {
    return LANGUAGE_LINK
        .matcher(page)
        .replaceAll(
            Matcher.quoteReplacement("href=\"?" + LocaleSelectorProvider.KC_LOCALE_PARAM + "="));
  }

  /**
   * The page's inline styles are the theme's and the realm's {@code loginCustomCss}, and the layout
   * needs them; the theme's scripts stay blocked.
   */
  private static String withStyleNonce(String page, String nonce) {
    return STYLE_TAG
        .matcher(page)
        .replaceAll(Matcher.quoteReplacement("<style nonce=\"" + nonce + "\""));
  }

  private Response.ResponseBuilder secure(Response.ResponseBuilder response, String nonce) {
    session.getProvider(org.keycloak.headers.SecurityHeadersProvider.class).options().skipHeaders();
    return response
        .type(MediaType.TEXT_HTML_TYPE.withCharset("UTF-8"))
        .header("Cache-Control", "no-store")
        .header("Pragma", "no-cache")
        .header("Referrer-Policy", "no-referrer")
        .header("X-Content-Type-Options", "nosniff")
        .header("X-Frame-Options", "DENY")
        .header("X-Robots-Tag", "none")
        .header("Strict-Transport-Security", strictTransportSecurity())
        .header(
            "Content-Security-Policy",
            contentSecurityPolicy(
                keycloakOrigin(),
                nonce,
                PinLinkConfig.assetOrigins(session.getContext().getRealm()::getAttribute)));
  }

  /** Skipping Keycloak's header set must not lose the realm's HSTS; a null header isn't sent. */
  private String strictTransportSecurity() {
    return session
        .getContext()
        .getRealm()
        .getBrowserSecurityHeaders()
        .get("strictTransportSecurity");
  }

  private String keycloakOrigin() {
    URI base = session.getContext().getUri().getBaseUri();
    return base.getScheme() + "://" + base.getAuthority();
  }

  /**
   * Keycloak's own origin is named because a short link's host, in front of a rewrite, may differ
   * from it. Nothing else is allowed to load, except images and fonts from the origins the realm
   * lists for its CSS. Those can't read the page, and a browser tells them at most this origin as
   * the referrer, never the path with the token. A script or stylesheet from another host could
   * read the PIN, so no attribute allows one.
   */
  static String contentSecurityPolicy(
      String keycloakOrigin, String nonce, List<String> assetOrigins) {
    String assets = assetOrigins.stream().map(origin -> " " + origin).collect(joining());
    return "default-src 'none'; script-src 'nonce-"
        + nonce
        + "'; style-src 'self' "
        + keycloakOrigin
        + " 'nonce-"
        + nonce
        + "'; img-src 'self' "
        + keycloakOrigin
        + " data:"
        + assets
        + "; font-src 'self' "
        + keycloakOrigin
        + assets
        + "; form-action 'self'; base-uri 'none'; frame-ancestors 'none'";
  }

  private static String randomToken() {
    byte[] bytes = new byte[18];
    RANDOM.nextBytes(bytes);
    return Base64.getUrlEncoder().withoutPadding().encodeToString(bytes);
  }
}
