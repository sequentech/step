package sequent.keycloak.authenticator.gateway;

import javax.mail.Message;
import javax.mail.MessagingException;
import javax.mail.Session;
import javax.mail.Transport;
import javax.mail.internet.InternetAddress;
import javax.mail.internet.MimeMessage;
import java.util.Properties;
import java.util.Map;

public class SmtpEmailService implements EmailService {

    private Session emailSession;

    public SmtpEmailService(Map<String, String> config) {
        Properties properties = new Properties();
        properties.put("mail.smtp.auth", "true");
        properties.put("mail.smtp.starttls.enable", "true");
        properties.put("mail.smtp.host", System.getenv("SMTP_HOST"));
        properties.put("mail.smtp.port", System.getenv("SMTP_PORT"));
        properties.put("mail.smtp.user", System.getenv("SMTP_USER"));
        properties.put("mail.smtp.password", System.getenv("SMTP_PASSWORD"));

        emailSession = Session.getInstance(properties);
    }

    @Override
    public void send(
		String emailAddress,
		String title,
		String body,
		String htmlBody
	) {
        try {
            MimeMessage message = new MimeMessage(emailSession);
            message.setFrom(new InternetAddress(System.getenv("SMTP_USER")));
            message.addRecipient(
				Message.RecipientType.TO,
				new InternetAddress(emailAddress)
			);
            message.setSubject(title);
            message.setText(body, "utf-8", "html");

            Transport transport = emailSession.getTransport("smtp");
            transport.connect(
				System.getenv("SMTP_HOST"), 
				System.getenv("SMTP_USER"), 
				System.getenv("SMTP_PASSWORD")
			);
            transport.sendMessage(message, message.getAllRecipients());
            transport.close();

        } catch (MessagingException e) {
            e.printStackTrace();
        }
    }
}
