// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.UUID;
import org.jboss.logging.Logger;
import org.keycloak.authentication.actiontoken.DefaultActionToken;
import sequent.keycloak.authenticator.MFAMethodSelector;

public class SmartLinkActionToken extends DefaultActionToken {
  private static final Logger logger = Logger.getLogger(MFAMethodSelector.class);

  public static final String TOKEN_TYPE = "smart-link";
  private static final String JSON_FIELD_MARK_EMAIL_VERIFIED = "mark-email-verified";
  private static final String JSON_FIELD_REDIRECT_URI = "redirect-uri";
  private static final String JSON_FIELD_SCOPES = "scopes";
  private static final String JSON_FIELD_STATE = "state";
  private static final String JSON_FIELD_REMEMBER_ME = "remember-me";
  private static final String JSON_FIELD_PERSISTENT = "persistent";

  @JsonProperty(value = JSON_FIELD_MARK_EMAIL_VERIFIED)
  private Boolean markEmailVerified = true;

  @JsonProperty(value = JSON_FIELD_REDIRECT_URI)
  private String redirectUri;

  @JsonProperty(value = JSON_FIELD_SCOPES)
  private String scopes;

  @JsonProperty(value = JSON_FIELD_STATE)
  private String state;

  @JsonProperty(value = JSON_FIELD_PERSISTENT)
  private Boolean persistent = true;

  @JsonProperty(value = JSON_FIELD_REMEMBER_ME)
  private Boolean rememberMe = false;

  public SmartLinkActionToken(
      String userId,
      int expirationInSecs,
      String nonce,
      String clientId,
      Boolean markEmailVerified,
      String redirectUri,
      String scopes,
      String state,
      Boolean rememberMe,
      Boolean persistent) {
    super(userId, TOKEN_TYPE, expirationInSecs, parseNonce(nonce));

    this.markEmailVerified = markEmailVerified;
    this.redirectUri = redirectUri;
    this.issuedFor = clientId;
    this.scopes = scopes;
    this.state = state;
    this.rememberMe = rememberMe;
    this.persistent = persistent;
  }

  private SmartLinkActionToken() {
    // we must have this private constructor for deserializer
  }

  static UUID parseNonce(String nonce) {
    try {
      return UUID.fromString(nonce);
    } catch (Exception error) {
      logger.error("error parsing nonce=" + error);
      return null;
    }
  }

  public Boolean getMarkEmailVerified() {
    return markEmailVerified;
  }

  public void setMarkEmailVerified(Boolean markEmailVerified) {
    this.markEmailVerified = markEmailVerified;
  }

  public String getRedirectUri() {
    return redirectUri;
  }

  public void setRedirectUri(String redirectUri) {
    this.redirectUri = redirectUri;
  }

  public String getScopes() {
    return this.scopes;
  }

  public void setScopes(String scopes) {
    this.scopes = scopes;
  }

  public String getState() {
    return this.state;
  }

  public void setState(String state) {
    this.state = state;
  }

  public Boolean getRememberMe() {
    return this.rememberMe;
  }

  public void setRememberMe(Boolean rememberMe) {
    this.rememberMe = rememberMe;
  }

  public Boolean getPersistent() {
    return this.persistent;
  }

  public void setPersistent(Boolean persistent) {
    this.persistent = persistent;
  }
}
