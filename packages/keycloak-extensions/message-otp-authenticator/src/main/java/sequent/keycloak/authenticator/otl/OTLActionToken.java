// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.otl;

import com.fasterxml.jackson.annotation.JsonProperty;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.actiontoken.DefaultActionToken;

/**
 * A One Time Link (OTL) token, typically sent via email/sms to verify the user has access to the
 * user's email/sms.
 */
@JBossLog
public class OTLActionToken extends DefaultActionToken {
  public static final String TOKEN_TYPE = "otl-action-token";
  private static final String JSON_FIELD_SESSION_ID = "original-compound-session-id";
  private static final String JSON_FIELD_AUTH_NOTE_NAMES = "auth-note-names";
  private static final String JSON_FIELD_IS_DEFERRED_USER = "is-deferred-user";
  private static final String JSON_FIELD_REDIRECT_URI = "redirect-uri";

  @JsonProperty(JSON_FIELD_SESSION_ID)
  private String originalCompoundSessionId;

  @JsonProperty(JSON_FIELD_REDIRECT_URI)
  private String redirectUri;

  @JsonProperty(JSON_FIELD_AUTH_NOTE_NAMES)
  private String[] authNoteNames;

  @JsonProperty(JSON_FIELD_IS_DEFERRED_USER)
  private boolean isDeferredUser;

  public OTLActionToken(
      String userId,
      int absoluteExpirationInSecs,
      String originalCompoundSessionId,
      String[] authNoteNames,
      boolean isDeferredUser,
      String redirectUri,
      String clientId) {
    super(userId, TOKEN_TYPE, absoluteExpirationInSecs, null, null);
    log.info(
        "OTLActionToken: userId="
            + userId
            + ", absoluteExpirationInSecs="
            + absoluteExpirationInSecs
            + ", originalCompoundSessionId="
            + originalCompoundSessionId
            + ", authNoteNames="
            + authNoteNames
            + ", isDeferredUser="
            + isDeferredUser
            + ", redirectUri="
            + redirectUri
            + ", clientId="
            + clientId);
    this.issuedFor = clientId;
    setOriginalCompoundSessionId(originalCompoundSessionId);
    setAuthNoteNames(authNoteNames);
    setIsDeferredUser(isDeferredUser);
    setRedirectUri(redirectUri);
  }

  OTLActionToken() {
    super();
    log.info("OTLActionToken private");
  }

  @Override
  public boolean isActive() {
    log.info("OTLActionToken isActive() => true");
    return true;
  }

  public String getOriginalCompoundSessionId() {
    log.info("getOriginalCompoundSessionId(): " + originalCompoundSessionId);
    return originalCompoundSessionId;
  }

  public void setOriginalCompoundSessionId(String originalCompoundSessionId) {
    log.info("setOriginalCompoundSessionId() = " + originalCompoundSessionId);
    this.originalCompoundSessionId = originalCompoundSessionId;
  }

  public String[] getAuthNoteNames() {
    log.info("getAuthNoteNames(): " + authNoteNames);
    return authNoteNames;
  }

  public void setAuthNoteNames(String[] authNoteNames) {
    log.info("setAuthNoteNames() = " + authNoteNames);
    this.authNoteNames = authNoteNames;
  }

  public boolean getIsDeferredUser() {
    log.info("getIsDeferredUser(): " + isDeferredUser);
    return isDeferredUser;
  }

  public void setIsDeferredUser(boolean isDeferredUser) {
    log.info("setIsDeferredUser() = " + isDeferredUser);
    this.isDeferredUser = isDeferredUser;
  }

  public void setRedirectUri(String redirectUri) {
    log.info("setRedirectUri() = " + redirectUri);
    this.redirectUri = redirectUri;
  }

  public String getRedirectUri() {
    return this.redirectUri;
  }
}
