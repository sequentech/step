// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resource.RealmResourceProviderFactory;

@JBossLog
@AutoService(RealmResourceProviderFactory.class)
public class ManualVerificationProviderFactory implements RealmResourceProviderFactory {
  static final String PROVIDER_ID = "manual-verification";

  @Override
  public RealmResourceProvider create(KeycloakSession session) {
    log.info("create");
    return new ManualVerificationProvider(session);
  }

  @Override
  public void init(Config.Scope config) {
    log.info("init");
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    log.info("postInit");
  }

  @Override
  public void close() {
    log.info("close");
  }

  @Override
  public String getId() {
    log.info("getId");
    return PROVIDER_ID;
  }

  // No-arg constructor is implicit
}
