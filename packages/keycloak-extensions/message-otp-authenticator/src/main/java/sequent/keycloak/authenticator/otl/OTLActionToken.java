// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.otl;

import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.actiontoken.DefaultActionToken;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * A One Time Link (OTL) token, typically sent via email/sms to verify
 * the user has access to the user's email/sms.
 */
@JBossLog
public class OTLActionToken extends DefaultActionToken {
    public static final String TOKEN_TYPE = "otl-action-token";
    private static final String JSON_FIELD_SESSION_ID = "original-compound-session-id";

    @JsonProperty(JSON_FIELD_SESSION_ID)
    private String originalCompoundSessionId;

    public OTLActionToken(
        String userId,
        int absoluteExpirationInSecs,
        String originalCompoundSessionId,
        String clientId
    ) {
        super(userId, TOKEN_TYPE, absoluteExpirationInSecs, null, null);
        log.info("OTLActionToken: userId=" + userId + ", absoluteExpirationInSecs=" + absoluteExpirationInSecs + ", originalCompoundSessionId=" + originalCompoundSessionId + ", clientId=" + clientId);
        this.issuedFor = clientId;
        setOriginalCompoundSessionId(originalCompoundSessionId);
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
}
