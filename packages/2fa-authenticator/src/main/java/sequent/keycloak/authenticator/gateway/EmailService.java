package sequent.keycloak.authenticator.gateway;

import java.util.Map;

public interface EmailService {
	void send(String emailAddress, String title, String body, String htmlBody);
}
