


/*
 * Unit Testing dummyemailsenderprovider  JC:Ayeng
 * Dummy email sender provider that just prints emails into the standard output
 * log.
*/

 import org.junit.jupiter.api.BeforeEach;
 import org.junit.jupiter.api.Test;
 import org.keycloak.email.EmailException;
 import uk.org.lidalia.slf4jtest.TestLogger;
 import uk.org.lidalia.slf4jtest.TestLoggerFactory;
 
 import java.util.HashMap;
 import java.util.Map;
 
 import static org.junit.jupiter.api.Assertions.assertTrue;
 import static uk.org.lidalia.slf4jtest.LoggingEvent.info;
 
 public class DummyEmailSenderProviderTest {
 
     private DummyEmailSenderProvider emailSenderProvider;
     private TestLogger logger;
 
     @BeforeEach
     public void setUp() {
         emailSenderProvider = new DummyEmailSenderProvider();
         logger = TestLoggerFactory.getTestLogger(DummyEmailSenderProvider.class);
     }
 
     @Test
     public void testSendLogsEmailDetails() throws EmailException {
         // Given
         Map<String, String> config = new HashMap<>();
         String address = "test@example.com";
         String subject = "Test Subject";
         String textBody = "This is a test email in plain text.";
         String htmlBody = "<p>This is a test email in HTML.</p>";
 
         // When
         emailSenderProvider.send(config, address, subject, textBody, htmlBody);
 
         System.out.println("Email sent - Subject: " + subject + ", Address: " + address + ", Text Body: " + textBody + ", HTML Body: " + htmlBody);

         // Then
         assertTrue(logger.getLoggingEvents().contains(
             info("**Sending dummy email**:\n\t- subject={}\n\t- address={}\n\t- textBody={}\n\t- htmlBody={}", subject, address, textBody, htmlBody)
         ));
         logger.debug("Email sent - Subject: {}, Address: {}, Text Body: {}, HTML Body: {}", subject, address, textBody, htmlBody);
 
     }
 }
 
 
 