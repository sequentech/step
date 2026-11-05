// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.google.auto.service.AutoService;
import java.util.Optional;
import java.util.function.UnaryOperator;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.provider.ProviderFactory;

/**
 * The sender of a Keycloak whose image was built without choosing one. {@code
 * spi-message-sender-provider} is a build option, so it cannot be changed once the image exists;
 * this factory decides with settings read when Keycloak starts and when a message is sent:
 *
 * <ul>
 *   <li>the harvest sender, when harvest has an address ({@code HARVEST_DOMAIN} or {@code
 *       --spi-message-sender-harvest-url});
 *   <li>unless the policy is {@link MessageSenderPolicy#KEYCLOAK_ONLY}, set for the deployment with
 *       {@code MESSAGE_SENDER_POLICY} (or {@code --spi-message-sender-auto-policy}) or for one
 *       realm with its {@code sequent.message-sender-policy} attribute. Either one is enough to
 *       keep every message in Keycloak; a realm cannot undo the deployment's choice.
 * </ul>
 *
 * A channel is still only used when the realm's {@code sequent.messaging} attribute offers it.
 */
@JBossLog
@AutoService(MessageSenderProviderFactory.class)
public class AutoMessageSenderProviderFactory implements MessageSenderProviderFactory {
  public static final String PROVIDER_ID = "auto";
  public static final String POLICY_OPTION = "policy";
  public static final String POLICY_ENV = "MESSAGE_SENDER_POLICY";
  public static final String POLICY_REALM_ATTRIBUTE = "sequent.message-sender-policy";

  private MessageSenderPolicy policy = MessageSenderPolicy.HARVEST_WHEN_CONFIGURED;
  private Optional<ProviderFactory<MessageSenderProvider>> harvest = Optional.empty();

  @Override
  public MessageSenderProvider create(KeycloakSession session) {
    if (policy == MessageSenderPolicy.KEYCLOAK_ONLY
        || realmPolicy(session).equals(Optional.of(MessageSenderPolicy.KEYCLOAK_ONLY))) {
      return new DefaultMessageSenderProvider();
    }
    return harvest
        .map(factory -> factory.create(session))
        .orElseGet(DefaultMessageSenderProvider::new);
  }

  private static Optional<MessageSenderPolicy> realmPolicy(KeycloakSession session) {
    RealmModel realm = session.getContext() == null ? null : session.getContext().getRealm();
    return realm == null
        ? Optional.empty()
        : MessageSenderPolicy.parse(realm.getAttribute(POLICY_REALM_ATTRIBUTE));
  }

  @Override
  public void init(Config.Scope config) {
    init(config, System::getenv);
  }

  void init(Config.Scope config, UnaryOperator<String> environment) {
    policy =
        MessageSenderPolicy.parse(config.get(POLICY_OPTION))
            .or(() -> MessageSenderPolicy.parse(environment.apply(POLICY_ENV)))
            .orElse(MessageSenderPolicy.HARVEST_WHEN_CONFIGURED);
    log.infov("init(): message sender policy {0}", policy);
  }

  MessageSenderPolicy policy() {
    return policy;
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    harvest =
        Optional.ofNullable(
            factory.getProviderFactory(
                MessageSenderProvider.class, HarvestMessageSenderProviderFactory.PROVIDER_ID));
  }

  @Override
  public void close() {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  /** The highest order wins when no provider is configured. */
  @Override
  public int order() {
    return 2;
  }
}
