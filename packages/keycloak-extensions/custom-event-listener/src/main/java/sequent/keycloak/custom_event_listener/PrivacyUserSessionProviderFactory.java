// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.common.util.MultiSiteUtils;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.UserSessionProvider;
import org.keycloak.models.UserSessionProviderFactory;
import org.keycloak.models.sessions.infinispan.PersistentUserSessionProvider;
import org.keycloak.models.sessions.infinispan.PrivacyPersistentUserSessionProvider;

/** Keeps the built-in provider responsible for cache, persistence and cluster lifecycle. */
@AutoService(UserSessionProviderFactory.class)
public final class PrivacyUserSessionProviderFactory
    implements UserSessionProviderFactory<UserSessionProvider> {
  public static final String ID = "sequent-privacy";
  private static final String DELEGATE = "infinispan";

  @Override
  public UserSessionProvider create(KeycloakSession session) {
    var delegate = session.getProvider(UserSessionProvider.class, DELEGATE);
    if (!(delegate instanceof PersistentUserSessionProvider persistent)) {
      throw new IllegalStateException(
          "The privacy session provider requires the persistent embedded Infinispan provider");
    }
    return new PrivacyPersistentUserSessionProvider(
        persistent,
        realm ->
            realm == null
                || realm.getName() == null
                || LogRealm.fromName(realm.getName()).kind() == LogRealm.Kind.EVENT,
        UserEventDetails::maySayNetwork);
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return ID;
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    requireSelected(factory);
    if (!MultiSiteUtils.isPersistentSessionsEnabled()) {
      throw new IllegalStateException(
          "The privacy session provider requires persistent user sessions");
    }
  }

  static void requireSelected(KeycloakSessionFactory factory) {
    if (!(factory.getProviderFactory(UserSessionProvider.class)
        instanceof PrivacyUserSessionProviderFactory)) {
      throw new IllegalStateException(
          "The sequent-privacy user session provider must be selected before authentication is served");
    }
    if (factory.getProviderFactory(UserSessionProvider.class, DELEGATE) == null) {
      throw new IllegalStateException(
          "The privacy session provider requires Infinispan; use spi-user-sessions--provider-default");
    }
  }

  @Override
  public void loadPersistentSessions(
      KeycloakSessionFactory factory, int maxErrors, int sessionsPerSegment) {
    var delegate = factory.getProviderFactory(UserSessionProvider.class, DELEGATE);
    if (!(delegate instanceof UserSessionProviderFactory<?> sessions)) {
      throw new IllegalStateException("The built-in session factory is unavailable");
    }
    sessions.loadPersistentSessions(factory, maxErrors, sessionsPerSegment);
  }
}
