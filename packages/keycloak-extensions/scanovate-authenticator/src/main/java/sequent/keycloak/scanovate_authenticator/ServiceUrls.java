// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.net.URI;
import java.net.URISyntaxException;
import java.util.Map;
import java.util.Set;

/** Base URLs of the on-premise Scanovate services, read from the authenticator configuration. */
final class ServiceUrls {
  private static final Set<String> SCHEMES = Set.of("http", "https");

  private ServiceUrls() {}

  /**
   * Reads an http or https base URL, without a trailing slash.
   *
   * @throws ScanovateException if the URL is missing or malformed
   */
  static URI parse(Map<String, String> config, String key) throws ScanovateException {
    String raw = config.getOrDefault(key, "").trim();
    URI url;
    try {
      url = new URI(raw.endsWith("/") ? raw.substring(0, raw.length() - 1) : raw);
    } catch (URISyntaxException e) {
      throw new ScanovateException("Invalid " + key + ": " + raw, e);
    }
    if (url.getScheme() == null || !SCHEMES.contains(url.getScheme()) || url.getHost() == null) {
      throw new ScanovateException("Invalid " + key + ": " + raw);
    }
    return url;
  }
}
