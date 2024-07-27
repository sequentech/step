// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import com.google.auto.service.AutoService;
import org.keycloak.provider.ProviderFactory;

@AutoService(ProviderFactory.class)
public interface SmsSenderProviderFactory extends ProviderFactory<SmsSenderProvider> {}
