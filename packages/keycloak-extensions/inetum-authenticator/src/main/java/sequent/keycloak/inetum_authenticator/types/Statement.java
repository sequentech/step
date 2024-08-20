package sequent.keycloak.inetum_authenticator.types;

public class Statement {
    private String head;
    private String body;

    public Statement(String head, String body) {
        this.head = head;
        this.body = body;
    }
}
