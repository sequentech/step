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

  @JsonProperty(JSON_FIELD_SESSION_ID)
  private String originalCompoundSessionId;

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
      String clientId) {
    super(userId, TOKEN_TYPE, absoluteExpirationInSecs, null, null);
    log.debug(
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
            + ", clientId="
            + clientId);
    this.issuedFor = clientId;
    setOriginalCompoundSessionId(originalCompoundSessionId);
    setAuthNoteNames(authNoteNames);
    setIsDeferredUser(isDeferredUser);
  }

  OTLActionToken() {
    super();
    log.debug("OTLActionToken private");
  }

  @Override
  public boolean isActive() {
    log.debug("OTLActionToken isActive() => true");
    return true;
  }

  public String getOriginalCompoundSessionId() {
    log.debug("getOriginalCompoundSessionId(): " + originalCompoundSessionId);
    return originalCompoundSessionId;
  }

  public void setOriginalCompoundSessionId(String originalCompoundSessionId) {
    log.debug("setOriginalCompoundSessionId() = " + originalCompoundSessionId);
    this.originalCompoundSessionId = originalCompoundSessionId;
  }

  public String[] getAuthNoteNames() {
    log.debug("getAuthNoteNames(): " + authNoteNames);
    return authNoteNames;
  }

  public void setAuthNoteNames(String[] authNoteNames) {
    log.debug("setAuthNoteNames() = " + authNoteNames);
    this.authNoteNames = authNoteNames;
  }

  public boolean getIsDeferredUser() {
    log.debug("getIsDeferredUser(): " + isDeferredUser);
    return isDeferredUser;
  }

  public void setIsDeferredUser(boolean isDeferredUser) {
    log.debug("setIsDeferredUser() = " + isDeferredUser);
    this.isDeferredUser = isDeferredUser;
  }
}
