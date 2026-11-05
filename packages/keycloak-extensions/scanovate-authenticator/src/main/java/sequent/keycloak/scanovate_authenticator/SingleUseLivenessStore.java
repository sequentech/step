// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Map;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.SingleUseObjectProvider;

/** {@link LivenessStore} backed by Keycloak's single-use object store, shared by the cluster. */
public class SingleUseLivenessStore implements LivenessStore {
  private final SingleUseObjectProvider provider;

  public SingleUseLivenessStore(KeycloakSession session) {
    this.provider = session.singleUseObjects();
  }

  @Override
  public void put(String key, long lifespanSeconds, Map<String, String> value) {
    provider.put(key, lifespanSeconds, value);
  }

  @Override
  public Map<String, String> get(String key) {
    return provider.get(key);
  }

  @Override
  public void remove(String key) {
    provider.remove(key);
  }
}
