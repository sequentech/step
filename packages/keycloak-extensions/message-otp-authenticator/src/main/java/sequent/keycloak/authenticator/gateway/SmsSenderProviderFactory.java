// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import org.keycloak.provider.ProviderFactory;
import com.google.auto.service.AutoService;


@AutoService(ProviderFactory.class)
public interface SmsSenderProviderFactory 
    extends ProviderFactory<SmsSenderProvider>
{
}
