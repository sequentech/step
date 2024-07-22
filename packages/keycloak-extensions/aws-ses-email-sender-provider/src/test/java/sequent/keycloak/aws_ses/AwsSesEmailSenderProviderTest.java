// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

  
package sequent.keycloak.aws_ses;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;
import org.mockito.junit.jupiter.MockitoExtension;
import software.amazon.awssdk.services.ses.SesClient;
import software.amazon.awssdk.services.ses.model.SendEmailRequest;
import software.amazon.awssdk.services.ses.model.SendEmailResponse;
import org.keycloak.email.EmailException;
import java.util.HashMap;
import java.util.Map;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
public class AwsSesEmailSenderProviderTest {

    @Mock
    private SesClient sesClientMock;

    @InjectMocks
    private AwsSesEmailSenderProvider emailSenderProvider;

    public AwsSesEmailSenderProviderTest(@Mock SesClient sesClientMock) {
        this.sesClientMock = sesClientMock;
        this.emailSenderProvider = new AwsSesEmailSenderProvider(sesClientMock);
    }

    @BeforeEach
    public void setUp() {
  
    }

    @Test
    public void testSendEmail() throws EmailException {
        // Mock SES response to simulate successful email sending
        SendEmailResponse mockResponse = SendEmailResponse.builder().messageId("mockMessageId").build();

        // Configure sesClientMock to return mockResponse when sendEmail is called
        when(sesClientMock.sendEmail(any(SendEmailRequest.class))).thenReturn(mockResponse);

        // Perform the test
        Map<String, String> config = new HashMap<>();
        config.put("from", "sender@example.com");
        config.put("fromDisplayName", "Sender Name");
        String address = "recipient@example.com";
        String subject = "Test Subject";
        String textBody = "Hello, this is a text email.";
        String htmlBody = "<html><body><h1>Hello</h1><p>This is an HTML email.</p></body></html>";

        // Call the method under test
        emailSenderProvider.send(config, address, subject, textBody, htmlBody);

        // Verify SES client interaction
        verify(sesClientMock, times(1)).sendEmail(any(SendEmailRequest.class));
    }


 
    @Test
    public void testSendEmailGeneralException() {
        // Test data
        Map<String, String> config = new HashMap<>();
        config.put("from", "sender@example.com");
        config.put("fromDisplayName", "Sender Name");
        String address = "recipient@example.com";
        String subject = "Test Subject";
        String textBody = "Hello, this is a text email.";
        String htmlBody = "<html><body><h1>Hello</h1><p>This is an HTML email.</p></body></html>";

        // Configure sesClientMock to throw a general Exception
        doThrow(new RuntimeException("General error")).when(sesClientMock).sendEmail(any(SendEmailRequest.class));

        // Perform the test and verify that EmailException is thrown
        EmailException thrown = assertThrows(EmailException.class, () -> {
            emailSenderProvider.send(config, address, subject, textBody, htmlBody);
        });

        // Verify that the exception message is as expected
        assertEquals("Exception: Failed to send email via AWS SES", thrown.getMessage());
    }
}
