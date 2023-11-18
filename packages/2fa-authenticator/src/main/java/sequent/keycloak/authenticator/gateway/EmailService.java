package sequent.keycloak.authenticator.gateway;

public interface EmailService {
	void send(String emailAddress, String title, String body, String htmlBody);
}
