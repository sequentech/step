// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import org.junit.jupiter.api.Test;
import org.keycloak.common.util.MultiSiteUtils;
import org.keycloak.events.EventStoreProvider;
import org.keycloak.events.EventStoreProviderFactory;
import org.keycloak.models.*;

class PrivacyUserSessionProviderTest {
  @Test
  void startupRequiresSelectedWrapperAndExplicitDelegate() {
    KeycloakSessionFactory factory = mock(KeycloakSessionFactory.class);
    assertThrows(
        IllegalStateException.class,
        () -> PrivacyUserSessionProviderFactory.requireSelected(factory));
    when(factory.getProviderFactory(UserSessionProvider.class))
        .thenReturn(new PrivacyUserSessionProviderFactory());
    assertThrows(
        IllegalStateException.class,
        () -> PrivacyUserSessionProviderFactory.requireSelected(factory));
    when(factory.getProviderFactory(UserSessionProvider.class, "infinispan"))
        .thenReturn(mock(UserSessionProviderFactory.class));
    assertDoesNotThrow(() -> PrivacyUserSessionProviderFactory.requireSelected(factory));
  }

  @Test
  void refusesAnUnsupportedSessionImplementationRatherThanRetainingAttribution() {
    KeycloakSession session = mock(KeycloakSession.class);
    when(session.getProvider(UserSessionProvider.class, "infinispan"))
        .thenReturn(mock(UserSessionProvider.class));
    assertThrows(
        IllegalStateException.class, () -> new PrivacyUserSessionProviderFactory().create(session));
  }

  @Test
  void startupRefusesVolatileSessions() {
    KeycloakSessionFactory factory = mock(KeycloakSessionFactory.class);
    when(factory.getProviderFactory(UserSessionProvider.class))
        .thenReturn(new PrivacyUserSessionProviderFactory());
    when(factory.getProviderFactory(UserSessionProvider.class, "infinispan"))
        .thenReturn(mock(UserSessionProviderFactory.class));
    try (var profile = mockStatic(MultiSiteUtils.class)) {
      var error =
          assertThrows(
              IllegalStateException.class,
              () -> new PrivacyUserSessionProviderFactory().postInit(factory));
      assertTrue(error.getMessage().contains("requires persistent user sessions"));
    }
  }

  @Test
  void listenerRefusesAnExclusiveSelectionThatUnloadsThePrivacySessionFactory() {
    KeycloakSessionFactory factory = mock(KeycloakSessionFactory.class);
    when(factory.getProviderFactory(EventStoreProvider.class))
        .thenReturn(new PrivacyEventStoreProviderFactory());
    when(factory.getProviderFactory(EventStoreProvider.class, "jpa"))
        .thenReturn(mock(EventStoreProviderFactory.class));
    when(factory.getProviderFactory(UserSessionProvider.class))
        .thenReturn(mock(UserSessionProviderFactory.class));
    var listener = new CustomEventListenerProviderFactory(mock(PgmqEventPublisher.class));
    try {
      var error = assertThrows(IllegalStateException.class, () -> listener.postInit(factory));
      assertTrue(
          error.getMessage().contains("sequent-privacy user session provider must be selected"));
    } finally {
      listener.close();
    }
  }
}
