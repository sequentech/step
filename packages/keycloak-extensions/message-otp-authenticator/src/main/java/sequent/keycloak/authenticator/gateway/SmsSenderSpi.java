// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import com.google.auto.service.AutoService;
import org.keycloak.provider.Provider;
import org.keycloak.provider.ProviderFactory;
import org.keycloak.provider.Spi;

@AutoService(Spi.class)
public class SmsSenderSpi implements Spi {

  @Override
  public boolean isInternal() {
    return true;
  }

  @Override
  public String getName() {
    return "smsSender";
  }

  @Override
  public Class<? extends Provider> getProviderClass() {
    return SmsSenderProvider.class;
  }

  @Override
  public Class<? extends ProviderFactory> getProviderFactoryClass() {
    return SmsSenderProviderFactory.class;
  }
}
