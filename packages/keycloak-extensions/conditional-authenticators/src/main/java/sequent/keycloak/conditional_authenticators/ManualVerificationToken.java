// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.fasterxml.jackson.annotation.JsonProperty;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.actiontoken.DefaultActionToken;

/*
 * A token used to manually verify an user
 */
@JBossLog
public class ManualVerificationToken extends DefaultActionToken {

  public static final String TOKEN_TYPE = "manual-verification-token";
  private static final String JSON_FIELD_REDIRECT_URI = "reduri";

  @JsonProperty(JSON_FIELD_REDIRECT_URI)
  private String redirectUri;

  public ManualVerificationToken(String userId, int absoluteExpirationInSecs, String redirectUri) {
    super(userId, TOKEN_TYPE, absoluteExpirationInSecs, null, null);
    log.info("ManualVerificationToken");
    setRedirectUri(redirectUri);
  }

  ManualVerificationToken() {
    super();
    log.info("ManualVerificationToken private");
  }

  public String getRedirectUri() {
    log.info("getRedirectUri(): " + redirectUri);
    return redirectUri;
  }

  public void setRedirectUri(String redirectUri) {
    log.info("setRedirectUri() = " + redirectUri);
    this.redirectUri = redirectUri;
  }
}
