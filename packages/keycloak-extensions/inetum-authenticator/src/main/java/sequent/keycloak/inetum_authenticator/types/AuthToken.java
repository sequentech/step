package sequent.keycloak.inetum_authenticator.types;

import com.fasterxml.jackson.annotation.JsonProperty;

public class AuthToken {
    private String access_token;
    private long expires_in;
    private long refresh_expires_in;
    private String token_type;
    private String id_token;
    @JsonProperty("not-before-policy")
    private long not_before_Policy;
    private String scope;

    public String getAccess_token() {
        return access_token;
    }
}
