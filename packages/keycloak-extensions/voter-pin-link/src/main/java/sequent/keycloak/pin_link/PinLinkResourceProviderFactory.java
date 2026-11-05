// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resource.RealmResourceProviderFactory;
import sequent.keycloak.custom_event_listener.RabbitMqEventPublisher;

@AutoService(RealmResourceProviderFactory.class)
public class PinLinkResourceProviderFactory implements RealmResourceProviderFactory {

  /** Path segment under {@code /realms/{realm}/}. */
  public static final String PROVIDER_ID = "pin-link";

  private volatile RabbitMqEventPublisher publisher;

  @Override
  public RealmResourceProvider create(KeycloakSession session) {
    return new PinLinkResourceProvider(
        session, this::publisher, () -> System.getenv("MASTER_SECRET"));
  }

  private RabbitMqEventPublisher publisher() {
    if (publisher == null) {
      synchronized (this) {
        if (publisher == null) {
          publisher = RabbitMqEventPublisher.fromEnvironment();
        }
      }
    }
    return publisher;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {
    if (publisher != null) {
      publisher.close();
    }
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
