// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import java.io.ByteArrayInputStream;
import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.List;
import javax.security.auth.x500.X500Principal;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;

/**
 * Reads the client certificate forwarded by nginx via the ssl-client-cert header, extracts the
 * issuer CN, and sets the auth note "cert-type" to the matching CN from the allowed list, or to
 * "not-allowed" if the CN is not in the list.
 *
 * <p>This authenticator runs first in the X.509 flow. Downstream conditional sub-flows use
 * Condition - Auth Note (cert-type = <CN>) to select the correct X509/Validate Username Form
 * configuration for each certificate type.
 */
@JBossLog
public class X509CertClassifierAuthenticator implements Authenticator {

  public static final X509CertClassifierAuthenticator SINGLETON =
      new X509CertClassifierAuthenticator();

  public static final String AUTH_NOTE_CERT_TYPE = "cert-type";
  public static final String CERT_TYPE_NOT_ALLOWED = "not-allowed";

  private static final String SSL_CLIENT_CERT_HEADER = "ssl-client-cert";

  /**
   * Returns the list of allowed certificate issuer CNs for the current realm.
   *
   * <p>TODO: Replace this mock with a call to harvest's REST endpoint that reads the
   * allowed_cert_types annotation from the election_event table in the sequent-core postgres
   * module. The endpoint should be scoped to the realm so that each election event can define its
   * own set of accepted certificate authorities.
   */
  private List<String> getAllowedCertTypes(AuthenticationFlowContext context) {
    return List.of(
        "AC FNMT Usuarios",
        "DNIE");
  }

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    String certHeader =
        context.getHttpRequest().getHttpHeaders().getHeaderString(SSL_CLIENT_CERT_HEADER);

    if (certHeader == null || certHeader.isBlank()) {
      log.infov("authenticate(): no {0} header present", SSL_CLIENT_CERT_HEADER);
      context.getAuthenticationSession().setAuthNote(AUTH_NOTE_CERT_TYPE, CERT_TYPE_NOT_ALLOWED);
      context.attempted();
      return;
    }

    X509Certificate cert = parseCert(certHeader);
    if (cert == null) {
      log.warnv("authenticate(): failed to parse certificate from {0} header", SSL_CLIENT_CERT_HEADER);
      context.getAuthenticationSession().setAuthNote(AUTH_NOTE_CERT_TYPE, CERT_TYPE_NOT_ALLOWED);
      context.attempted();
      return;
    }

    String issuerCn = extractCn(cert.getIssuerX500Principal());
    log.infov("authenticate(): issuer CN={0}", issuerCn);

    List<String> allowed_cert_types = getAllowedCertTypes(context);
    boolean isAllowed = issuerCn != null && allowed_cert_types.contains(issuerCn);

    String certType = isAllowed ? issuerCn : CERT_TYPE_NOT_ALLOWED;
    log.infov("authenticate(): setting auth note {0}={1}", AUTH_NOTE_CERT_TYPE, certType);
    context.getAuthenticationSession().setAuthNote(AUTH_NOTE_CERT_TYPE, certType);
    context.attempted();
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
