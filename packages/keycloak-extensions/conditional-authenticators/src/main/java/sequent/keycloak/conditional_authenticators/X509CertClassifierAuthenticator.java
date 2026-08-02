// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static sequent.keycloak.authenticator.Utils.AUTH_NOTE_DENY_TYPE;
import static sequent.keycloak.authenticator.Utils.CA_CERT_ISSUER_CN;
import static sequent.keycloak.authenticator.Utils.CERT_NOT_PROVIDED;
import static sequent.keycloak.authenticator.Utils.VOTER_CERT_SUBJECT_DN;

import java.io.ByteArrayInputStream;
import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import javax.security.auth.x500.X500Principal;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;

/**
 * Reads the client certificate forwarded by a reverse proxy, extracts the issuer CN, and sets the
 * auth note "cert-type" to that CN value. If no certificate is present or it cannot be parsed, sets
 * "cert-type" to "not-allowed".
 *
 * <p>The header name is configurable via the authenticator config key {@code cert-header-name} (see
 * {@link X509CertClassifierAuthenticatorFactory#CONF_CERT_HEADER_NAME}). It defaults to {@code
 * ssl-client-cert} (nginx default) but can be set to {@code Cf-Tls-Client-Cert} for Cloudflare or
 * {@code Client-Cert} for RFC 9440 proxies.
 *
 * <p>This authenticator runs first in the X.509 flow. Downstream conditional sub-flows use
 * Condition - Auth Note (cert-type = &lt;CN&gt;) to select the correct X509/Validate Username Form
 * configuration for each certificate type.
 */
@JBossLog
public class X509CertClassifierAuthenticator implements Authenticator {

  /** Default HTTP header name when no authenticator config is provided. */
  public static final String DEFAULT_CERT_HEADER = "ssl-client-cert";

  public static final String AUTH_NOTE_CERT_TYPE = "cert-type";
  public static final String CERT_TYPE_NONE = "none";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    String headerName = resolveHeaderName(context);
    String certHeader = context.getHttpRequest().getHttpHeaders().getHeaderString(headerName);

    if (certHeader == null || certHeader.isBlank()) {
      log.infov(
          "authenticate(): no {0} header present, setting {1}={2}",
          headerName, AUTH_NOTE_DENY_TYPE, CERT_NOT_PROVIDED);
      context.getAuthenticationSession().setAuthNote(AUTH_NOTE_DENY_TYPE, CERT_NOT_PROVIDED);
      context.getEvent().detail(AUTH_NOTE_DENY_TYPE, CERT_NOT_PROVIDED);
      context.getEvent().detail(VOTER_CERT_SUBJECT_DN, CERT_TYPE_NONE);
      context.getEvent().detail(CA_CERT_ISSUER_CN, CERT_TYPE_NONE);
      context.attempted();
      return;
    }

    X509Certificate cert = parseCert(certHeader);
    if (cert == null) {
      log.warnv(
          "authenticate(): failed to parse certificate from {0} header, setting {1}={2}",
          headerName, AUTH_NOTE_DENY_TYPE, CERT_NOT_PROVIDED);
      context.getAuthenticationSession().setAuthNote(AUTH_NOTE_DENY_TYPE, CERT_NOT_PROVIDED);
      context.getEvent().detail(AUTH_NOTE_DENY_TYPE, CERT_NOT_PROVIDED);
      context.getEvent().detail(VOTER_CERT_SUBJECT_DN, CERT_TYPE_NONE);
      context.getEvent().detail(CA_CERT_ISSUER_CN, CERT_TYPE_NONE);
      context.attempted();
      return;
    }

    String issuerCn = extractCn(cert.getIssuerX500Principal());
    String certType = issuerCn != null ? issuerCn : CERT_TYPE_NONE;
    log.infov("authenticate(): setting auth note {0}={1}", AUTH_NOTE_CERT_TYPE, certType);
    context.getAuthenticationSession().setAuthNote(AUTH_NOTE_CERT_TYPE, certType);
    context.getEvent().detail(VOTER_CERT_SUBJECT_DN, cert.getSubjectX500Principal().getName());
    context.getEvent().detail(CA_CERT_ISSUER_CN, certType);
    context.success();
  }

  private String resolveHeaderName(AuthenticationFlowContext context) {
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    if (config != null && config.getConfig() != null) {
      String configured =
          config.getConfig().get(X509CertClassifierAuthenticatorFactory.CONF_CERT_HEADER_NAME);
      if (configured != null && !configured.isBlank()) {
        return configured.trim();
      }
    }
    return DEFAULT_CERT_HEADER;
  }

  private X509Certificate parseCert(String headerValue) {
    try {
      String decoded = URLDecoder.decode(headerValue, StandardCharsets.UTF_8);
      byte[] pemBytes = decoded.getBytes(StandardCharsets.UTF_8);
      CertificateFactory cf = CertificateFactory.getInstance("X.509");
      return (X509Certificate) cf.generateCertificate(new ByteArrayInputStream(pemBytes));
    } catch (Exception e) {
      log.warnv("parseCert(): {0}", e.getMessage());
      return null;
    }
  }

  private String extractCn(X500Principal principal) {
    if (principal == null) {
      return null;
    }
    // RFC 2253 name: "CN=AC FNMT Usuarios, OU=..., O=..., C=ES"
    for (String part : principal.getName().split(",")) {
      String trimmed = part.trim();
      if (trimmed.startsWith("CN=")) {
        return trimmed.substring(3);
      }
    }
    return null;
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    // Not used
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    // Not used
  }

  @Override
  public void close() {
    // Does nothing
  }
}
