// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Map;

/**
 * Where the Liveness Plus sessions are kept between the voter's browser, the Liveness Plus service
 * and Keycloak. It must be shared by every Keycloak node, see {@link SingleUseLivenessStore}.
 */
public interface LivenessStore {
  /** Stores (or overwrites) an entry, expiring after the given time. */
  void put(String key, long lifespanSeconds, Map<String, String> value);

  /** The entry, or null if there is none. */
  Map<String, String> get(String key);

  void remove(String key);
}
