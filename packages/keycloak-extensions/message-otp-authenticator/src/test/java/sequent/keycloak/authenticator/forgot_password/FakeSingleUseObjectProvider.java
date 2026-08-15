// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import java.util.HashMap;
import java.util.Map;
import org.keycloak.models.SingleUseObjectProvider;

/**
 * In-memory {@link SingleUseObjectProvider} test double. Real deployments back this SPI with
 * Infinispan (replicated across Keycloak nodes) so entries expire after their given lifespan; this
 * fake tracks a controllable virtual clock instead of wall-clock time, via {@link
 * #advanceTimeSeconds}, so tuple-throttle TTL expiry can be tested deterministically without
 * sleeping.
 */
final class FakeSingleUseObjectProvider implements SingleUseObjectProvider {

  private record Entry(Map<String, String> notes, long expiresAtSeconds) {}

  private final Map<String, Entry> store = new HashMap<>();
  private long nowSeconds = 0;

  void advanceTimeSeconds(long seconds) {
    nowSeconds += seconds;
  }

  @Override
  public void put(String key, long lifespanSeconds, Map<String, String> notes) {
    store.put(key, new Entry(notes, nowSeconds + lifespanSeconds));
  }

  @Override
  public Map<String, String> get(String key) {
    Entry entry = store.get(key);
    if (entry == null) {
      return null;
    }
    if (entry.expiresAtSeconds() <= nowSeconds) {
      store.remove(key);
      return null;
    }
    return entry.notes();
  }

  @Override
  public Map<String, String> remove(String key) {
    Entry entry = store.remove(key);
    return entry == null ? null : entry.notes();
  }

  @Override
  public boolean replace(String key, Map<String, String> notes) {
    Entry existing = store.get(key);
    if (existing == null || existing.expiresAtSeconds() <= nowSeconds) {
      return false;
    }
    store.put(key, new Entry(notes, existing.expiresAtSeconds()));
    return true;
  }

  @Override
  public boolean putIfAbsent(String key, long lifespanInSeconds) {
    if (get(key) != null) {
      return false;
    }
    store.put(key, new Entry(Map.of(), nowSeconds + lifespanInSeconds));
    return true;
  }

  @Override
  public boolean contains(String key) {
    return get(key) != null;
  }

  @Override
  public void close() {}
}
