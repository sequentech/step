// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.google.auto.service.AutoService;
import org.keycloak.provider.Provider;
import org.keycloak.provider.ProviderFactory;
import org.keycloak.provider.Spi;

/** Selected with {@code --spi-message-sender-provider=default|harvest}; defaults to "default". */
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
