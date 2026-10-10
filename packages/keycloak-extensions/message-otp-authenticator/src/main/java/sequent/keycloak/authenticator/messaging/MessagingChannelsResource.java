// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.util.List;
import java.util.Map;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resource.RealmResourceProviderFactory;

/**
 * {@code GET /realms/{realm}/messaging-channels}: the channels the event offers voters for codes,
 * from its public projection ({@code sequent.messaging}), as {@code {"code_channels": [...]}} in
 * the ticket's order. The voting portal shows "Change how we contact you" only when the list isn't
 * empty. Public data (the enrollment page shows the same channels to anyone), so no token is needed
 * and any origin may read it; nothing about voters or accounts is returned.
 */
public class MessagingChannelsResource implements RealmResourceProvider {
  public static final String PROVIDER_ID = "messaging-channels";

  private final KeycloakSession session;

  public MessagingChannelsResource(KeycloakSession session) {
    this.session = session;
  }

  @Override
  public Object getResource() {
    return this;
  }

  @GET
  @Produces(MediaType.APPLICATION_JSON)
  public Response channels() {
    RealmModel realm = session.getContext().getRealm();
    return Response.ok(Map.of("code_channels", codeChannels(realm)))
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-store")
        .build();
  }

  /** The event's code channels, by name; empty for an event without messaging. */
  static List<String> codeChannels(RealmModel realm) {
    return PublicMessagingChannels.fromRealm(realm)
        .map(projection -> projection.channelsWithPurpose(MessagePurpose.OTP))
        .map(VoterChannels::names)
        .orElse(List.of());
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
