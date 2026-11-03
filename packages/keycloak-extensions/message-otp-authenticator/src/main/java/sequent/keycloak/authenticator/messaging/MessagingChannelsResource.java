// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.OPTIONS;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.HttpHeaders;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.services.managers.AppAuthManager;
import org.keycloak.services.managers.AuthenticationManager;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resource.RealmResourceProviderFactory;
import sequent.keycloak.authenticator.ContactChange;

/**
 * {@code GET /realms/{realm}/messaging-channels}: whether "Change how we contact you" is offered,
 * as {@code {"code_channels": [...], "reason": "..."}} with the channels in the ticket's order and
 * the reason only when none is offered. The voting portal shows the entry only when the list isn't
 * empty.
 *
 * <ul>
 *   <li>With the voter's access token ({@code Authorization: Bearer}), the answer is the action's
 *       own decision for that voter ({@link ContactChange#forVoter}): the channels their Post
 *       offers, so the entry is shown exactly when the action would run (R7-14).
 *   <li>Without one (or with a token that doesn't authenticate), the event's code channels from its
 *       public projection ({@code sequent.messaging}): public data the enrollment page shows to
 *       anyone.
 * </ul>
 *
 * Either way, none is offered while a code step of the realm doesn't send to the voter's channel
 * (R7-01). Any origin may read it; nothing about other voters or accounts is returned.
 */
public class MessagingChannelsResource implements RealmResourceProvider {
  public static final String PROVIDER_ID = "messaging-channels";

  private final KeycloakSession session;
  private final Function<KeycloakSession, Optional<UserModel>> bearer;

  public MessagingChannelsResource(KeycloakSession session) {
    this(session, MessagingChannelsResource::bearerUser);
  }

  /**
   * @param bearer the voter the request's access token authenticates, if any
   */
  MessagingChannelsResource(
      KeycloakSession session, Function<KeycloakSession, Optional<UserModel>> bearer) {
    this.session = session;
    this.bearer = bearer;
  }

  @Override
  public Object getResource() {
    return this;
  }

  @GET
  @Produces(MediaType.APPLICATION_JSON)
  public Response channels() {
    RealmModel realm = session.getContext().getRealm();
    Optional<UserModel> voter = bearer.apply(session);
    ContactChange.Decision decision =
        voter
            .map(user -> ContactChange.forVoter(session, realm, user))
            .orElseGet(() -> ContactChange.forEvent(realm));
    Map<String, Object> body = new LinkedHashMap<>();
    body.put("code_channels", VoterChannels.names(decision.channels()));
    decision.refusal().ifPresent(reason -> body.put("reason", reason.reason()));
    return cors(Response.ok(body)).header("Cache-Control", "no-store").build();
  }

  /** The CORS preflight a request with the voter's token needs. */
  @OPTIONS
  public Response preflight() {
    return cors(Response.noContent())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .header("Access-Control-Allow-Headers", "Authorization, Accept")
        .header("Access-Control-Max-Age", "600")
        .build();
  }

  private static Response.ResponseBuilder cors(Response.ResponseBuilder response) {
    return response.header("Access-Control-Allow-Origin", "*");
  }

  /** The event's code channels, by name; empty for an event without messaging. */
  static List<String> codeChannels(RealmModel realm) {
    return VoterChannels.names(ContactChange.forEvent(realm).channels());
  }

  /** The voter a bearer access token authenticates; empty without a valid one. */
  static Optional<UserModel> bearerUser(KeycloakSession session) {
    try {
      HttpHeaders headers = session.getContext().getRequestHeaders();
      if (headers == null || AppAuthManager.extractAuthorizationHeaderToken(headers) == null) {
        return Optional.empty();
      }
      AuthenticationManager.AuthResult result =
          new AppAuthManager.BearerTokenAuthenticator(session)
              .setRealm(session.getContext().getRealm())
              .authenticate();
      return Optional.ofNullable(result).map(AuthenticationManager.AuthResult::user);
    } catch (RuntimeException e) {
      // An expired or foreign token: answered as for anyone.
      return Optional.empty();
    }
  }

  @Override
  public void close() {}

  /** Registers the resource under the realm. */
  @AutoService(RealmResourceProviderFactory.class)
  public static class Factory implements RealmResourceProviderFactory {
    @Override
    public RealmResourceProvider create(KeycloakSession session) {
      return new MessagingChannelsResource(session);
    }

    @Override
    public void init(Config.Scope config) {}

    @Override
    public void postInit(KeycloakSessionFactory factory) {}

    @Override
    public void close() {}

    @Override
    public String getId() {
      return PROVIDER_ID;
    }
  }
}
