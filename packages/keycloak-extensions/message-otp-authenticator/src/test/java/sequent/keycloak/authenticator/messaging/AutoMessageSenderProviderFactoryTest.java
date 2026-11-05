// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.util.EnumSet;
import java.util.HashMap;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.Test;
import org.keycloak.Config;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.provider.ProviderFactory;
import org.keycloak.services.DefaultKeycloakSessionFactory;

class AutoMessageSenderProviderFactoryTest {
  private static final Set<MessageChannel> MESSAGING_APPS =
      EnumSet.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER);

  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);

  private Config.Scope scope(Map<String, String> values) {
    Config.Scope scope = mock(Config.Scope.class);
    values.forEach((key, value) -> when(scope.get(key)).thenReturn(value));
    return scope;
  }

  /** The sender Keycloak uses when nothing selects a provider, started with these settings. */
  private MessageSenderProvider sender(
      Map<String, String> harvestOptions, Map<String, String> autoOptions) {
    HarvestMessageSenderProviderFactory harvest = new HarvestMessageSenderProviderFactory();
    harvest.init(scope(harvestOptions));
    KeycloakSessionFactory factory = mock(KeycloakSessionFactory.class);
    when(factory.getProviderFactory(
            MessageSenderProvider.class, HarvestMessageSenderProviderFactory.PROVIDER_ID))
        .thenReturn(harvest);
    AutoMessageSenderProviderFactory auto = new AutoMessageSenderProviderFactory();
    auto.init(scope(autoOptions), name -> null);
    auto.postInit(factory);
    KeycloakContext context = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(context);
    when(context.getRealm()).thenReturn(realm);
    return auto.create(session);
  }

  @Test
  void theRuntimeSelectionIsTheSenderOfAnImageBuiltWithoutOptions() {
    Map<String, ProviderFactory> factories = new HashMap<>();
    for (MessageSenderProviderFactory factory :
        new MessageSenderProviderFactory[] {
          new DefaultMessageSenderProviderFactory(),
          new HarvestMessageSenderProviderFactory(),
          new AutoMessageSenderProviderFactory()
        }) {
      factories.put(factory.getId(), factory);
    }
    assertEquals(
        AutoMessageSenderProviderFactory.PROVIDER_ID,
        DefaultKeycloakSessionFactory.resolveDefaultProvider(factories, new MessageSenderSpi()));
  }

  @Test
  void harvestConfiguredAtRuntimeDeliversTheMessagingApps() {
    MessageSenderProvider sender = sender(Map.of("url", "http://harvest.test:8400"), Map.of());
    assertEquals(MESSAGING_APPS, sender.getChannels());
    assertTrue(sender instanceof HarvestMessageSenderProvider);
  }

  @Test
  void withoutHarvestNothingIsDeliveredByTheSender() {
    MessageSenderProvider sender = sender(Map.of("url", ""), Map.of());
    assertEquals(Set.of(), sender.getChannels());
  }

  @Test
  void theDeploymentCanKeepEveryMessageInKeycloak() {
    MessageSenderProvider sender =
        sender(Map.of("url", "http://harvest.test:8400"), Map.of("policy", "KEYCLOAK_ONLY"));
    assertEquals(Set.of(), sender.getChannels());
  }

  @Test
  void theEnvironmentSetsThePolicyWhenNoOptionDoes() {
    AutoMessageSenderProviderFactory auto = new AutoMessageSenderProviderFactory();
    auto.init(
        scope(Map.of()),
        name -> AutoMessageSenderProviderFactory.POLICY_ENV.equals(name) ? "keycloak_only" : null);
    assertEquals(MessageSenderPolicy.KEYCLOAK_ONLY, auto.policy());

    auto.init(scope(Map.of("policy", "HARVEST_WHEN_CONFIGURED")), name -> "KEYCLOAK_ONLY");
    assertEquals(MessageSenderPolicy.HARVEST_WHEN_CONFIGURED, auto.policy());

    auto.init(scope(Map.of()), name -> null);
    assertEquals(MessageSenderPolicy.HARVEST_WHEN_CONFIGURED, auto.policy());
  }

  @Test
  void anUnreadablePolicyKeepsEveryMessageInKeycloak() {
    MessageSenderProvider sender =
        sender(Map.of("url", "http://harvest.test:8400"), Map.of("policy", "OFF"));
    assertEquals(Set.of(), sender.getChannels());
  }

  @Test
  void aRealmCanKeepItsMessagesInKeycloak() {
    when(realm.getAttribute(AutoMessageSenderProviderFactory.POLICY_REALM_ATTRIBUTE))
        .thenReturn("KEYCLOAK_ONLY");
    MessageSenderProvider sender = sender(Map.of("url", "http://harvest.test:8400"), Map.of());
    assertEquals(Set.of(), sender.getChannels());
  }

  @Test
  void aRealmCannotOverrideTheDeployment() {
    when(realm.getAttribute(AutoMessageSenderProviderFactory.POLICY_REALM_ATTRIBUTE))
        .thenReturn("HARVEST_WHEN_CONFIGURED");
    MessageSenderProvider sender =
        sender(Map.of("url", "http://harvest.test:8400"), Map.of("policy", "KEYCLOAK_ONLY"));
    assertEquals(Set.of(), sender.getChannels());
  }

  @Test
  void theConfiguredProviderIdsKeepTheirMeaning() {
    assertEquals("default", DefaultMessageSenderProviderFactory.PROVIDER_ID);
    assertEquals("harvest", HarvestMessageSenderProviderFactory.PROVIDER_ID);
    assertEquals(Set.of(), new DefaultMessageSenderProviderFactory().create(session).getChannels());
  }
}
