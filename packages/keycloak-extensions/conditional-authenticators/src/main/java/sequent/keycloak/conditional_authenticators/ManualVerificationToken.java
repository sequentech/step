package sequent.keycloak.conditional_authenticators;

import org.keycloak.authentication.actiontoken.DefaultActionToken;

import lombok.extern.jbosslog.JBossLog;

/*
 * A token used to manually verify an user 
 */
@JBossLog
public class ManualVerificationToken extends DefaultActionToken {

    public static final String TOKEN_TYPE = "manual-verification-token";

    public ManualVerificationToken(
        String userId,
        int absoluteExpirationInSecs,
        String compoundAuthenticationSessionId
    ) {
        super(
            userId,
            TOKEN_TYPE,
            absoluteExpirationInSecs,
            null,
            compoundAuthenticationSessionId
        );
        log.info("ManualVerificationToken");
    }

    private ManualVerificationToken() {
        // Required to deserialize from JWT
        super();
        log.info("ManualVerificationToken private");
    }
}
