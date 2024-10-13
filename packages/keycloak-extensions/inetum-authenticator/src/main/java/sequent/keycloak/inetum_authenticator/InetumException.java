package sequent.keycloak.inetum_authenticator;

public class InetumException extends Exception {
    private String error;

    public String getError() {
        return error;
    }

    public InetumException(String ftlErrorAuthInvalid) {
        this.error = ftlErrorAuthInvalid;
    }

}
