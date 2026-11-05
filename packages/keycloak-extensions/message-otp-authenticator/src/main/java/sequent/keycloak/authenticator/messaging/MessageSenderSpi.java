// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.google.auto.service.AutoService;
import org.keycloak.provider.Provider;
import org.keycloak.provider.ProviderFactory;
import org.keycloak.provider.Spi;

/**
 * With no provider configured the sender is {@code auto}, which decides when Keycloak runs (see
 * {@link AutoMessageSenderProviderFactory}). {@code --spi-message-sender-provider=default|harvest}
 * is a build option: it only takes effect when the image is built with it, and then pins the
 * sender.
 */
@AutoService(Spi.class)
public class MessageSenderSpi implements Spi {
  public static final String NAME = "messageSender";

  @Override
  public boolean isInternal() {
    return true;
  }

  @Override
  public String getName() {
    return NAME;
  }

  @Override
  public Class<? extends Provider> getProviderClass() {
    return MessageSenderProvider.class;
  }

  @Override
  public Class<? extends ProviderFactory> getProviderFactoryClass() {
    return MessageSenderProviderFactory.class;
  }
}
