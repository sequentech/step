// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.List;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.provider.ProviderConfigProperty;

/**
 * Decides whether the client certificate forwarded in a request header may be used, according to
 * the {@code cert-header-trust-policy} authenticator config. Shared by {@link
 * X509CertClassifierAuthenticator} and {@link X509UserResolutionAuthenticator}.
 */
@JBossLog
final class X509CertHeaderTrust {

  static final String CONF_TRUST_POLICY = "cert-header-trust-policy";
  static final String CONF_PROXY_SECRET = "cert-proxy-secret";

  /** Header the TLS-terminating proxy sets to the configured secret. */
  static final String PROXY_SECRET_HEADER = "X-Client-Cert-Proxy-Secret";

  enum Policy {
    /** Use the forwarded certificate header on every request. */
    ANY_REQUEST,
    /**
     * Use the forwarded certificate header only when the request also carries {@link
     * #PROXY_SECRET_HEADER} with the configured {@code cert-proxy-secret}.
     */
    REQUIRE_PROXY_SECRET;

    /** A missing value keeps {@link #ANY_REQUEST}; an unrecognised one is treated strictly. */
    static Policy fromConfig(String value) {
      if (value == null || value.isBlank()) {
        return ANY_REQUEST;
      }
      for (Policy policy : values()) {
        if (policy.name().equalsIgnoreCase(value.trim())) {
          return policy;
        }
      }
      return REQUIRE_PROXY_SECRET;
    }
  }

  private X509CertHeaderTrust() {}

  static boolean isTrusted(AuthenticationFlowContext context) {
    AuthenticatorConfigModel configModel = context.getAuthenticatorConfig();
    Map<String, String> config =
        configModel != null && configModel.getConfig() != null ? configModel.getConfig() : Map.of();
    Policy policy = Policy.fromConfig(config.get(CONF_TRUST_POLICY));
    if (policy == Policy.ANY_REQUEST) {
      return true;
    }

    String expected = config.get(CONF_PROXY_SECRET);
    if (expected == null || expected.isEmpty()) {
      log.warnv(
          "isTrusted(): {0}={1} but {2} is not configured",
          CONF_TRUST_POLICY, policy, CONF_PROXY_SECRET);
      return false;
    }
    String presented =
        context.getHttpRequest().getHttpHeaders().getHeaderString(PROXY_SECRET_HEADER);
    boolean trusted =
        presented != null
            && MessageDigest.isEqual(
                expected.getBytes(StandardCharsets.UTF_8),
                presented.getBytes(StandardCharsets.UTF_8));
    if (!trusted) {
      log.debugv(
          "isTrusted(): {0} missing or not matching, ignoring the client certificate header",
          PROXY_SECRET_HEADER);
    }
    return trusted;
  }

  static List<ProviderConfigProperty> configProperties() {
    ProviderConfigProperty policy =
        new ProviderConfigProperty(
            CONF_TRUST_POLICY,
            "Client Certificate Header Trust Policy",
            "ANY_REQUEST (default) uses the forwarded client certificate header on every request."
                + " REQUIRE_PROXY_SECRET uses it only when the request also carries the "
                + PROXY_SECRET_HEADER
                + " header with the configured proxy secret, which only the TLS-terminating proxy"
                + " that requests client certificates should send.",
            ProviderConfigProperty.LIST_TYPE,
            Policy.ANY_REQUEST.name());
    policy.setOptions(List.of(Policy.ANY_REQUEST.name(), Policy.REQUIRE_PROXY_SECRET.name()));

    ProviderConfigProperty secret =
        new ProviderConfigProperty(
            CONF_PROXY_SECRET,
            "Client Certificate Proxy Secret",
            "Value the TLS-terminating proxy sends in the "
                + PROXY_SECRET_HEADER
                + " header. Used only with REQUIRE_PROXY_SECRET.",
            ProviderConfigProperty.PASSWORD,
            null);
    secret.setSecret(true);

    return List.of(policy, secret);
  }
}
