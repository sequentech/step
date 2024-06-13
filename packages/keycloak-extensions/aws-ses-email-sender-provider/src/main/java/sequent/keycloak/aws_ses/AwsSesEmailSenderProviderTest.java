package sequent.keycloak.aws_ses;
/*
 * Unit Concitional AuthNote Authenticator  JC:Ayeng  6132024
 *
 * NOTE : Commented out so dependencies are installed properly for other Unit Testing Files.
 *         UNCOMMENT IF TESTING FOR THIS FILE ONLY, or dependencies will be needed in all extensions
* /

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;
import software.amazon.awssdk.services.ses.SesClient;
import software.amazon.awssdk.services.ses.model.SendEmailRequest;
import software.amazon.awssdk.services.ses.model.SendEmailResponse;
import org.keycloak.email.EmailException;

import java.util.HashMap;
import java.util.Map;

import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

public class AwsSesEmailSenderProviderTest {

    @Mock
    private SesClient sesClientMock;

    private AwsSesEmailSenderProvider emailSenderProvider;

    @BeforeEach
    public void setUp() {
        MockitoAnnotations.openMocks(this);
        emailSenderProvider = new AwsSesEmailSenderProvider(sesClientMock);
    }

    @Test
    public void testSendEmail() throws EmailException {
        // Given
        String address = "recipient@example.com";
        String subject = "Test Subject";
        String textBody = "Test text body";
        String htmlBody = "<p>Test HTML body</p>";
  Map<String, String> config = new HashMap<>();
        config.put("from", "sender@example.com");

        SendEmailResponse sendEmailResponse = SendEmailResponse.builder().build();
        when(sesClientMock.sendEmail(any(SendEmailRequest.class))).thenReturn(sendEmailResponse);

        // When
        emailSenderProvider.send(config, address, subject, textBody, htmlBody);

        // Then
        verify(sesClientMock, times(1)).sendEmail(any(SendEmailRequest.class));
    }
}
    */