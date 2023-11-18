package sequent.keycloak.authenticator.credential;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;

public class MessageOTPCredentialData {
    private final boolean isSetup;

    @JsonCreator
    public MessageOTPCredentialData(@JsonProperty("isSetup") boolean isSetup) {
        this.isSetup = isSetup;
    }

    public boolean isSetup() {
        return this.isSetup;
    }
}

