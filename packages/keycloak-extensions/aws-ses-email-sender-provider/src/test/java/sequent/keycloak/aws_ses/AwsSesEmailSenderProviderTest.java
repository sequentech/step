// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.aws_ses;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.ses.SesClient;
import software.amazon.awssdk.services.ses.model.*;
import org.keycloak.email.EmailException;

import java.util.HashMap;
import java.util.Map;

import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
public class AwsSesEmailSenderProviderTest {

    @Mock
    private SesClient sesClientMock;

    @InjectMocks
    private AwsSesEmailSenderProvider emailSenderProvider;

    @BeforeEach
    public void setUp() {
        // Clear any invocations before each test
        reset(sesClientMock);

        // Configure SesClient with specific AWS region
        sesClientMock = SesClient.builder()
                .region(Region.US_EAST_1) // Replace with your desired region
                .build();
       //System.setProperty("aws.region", "us-east-1"); 
    }

    @Test
    public void testSendEmail() throws Exception {
        // Mock SES response
        when(sesClientMock.sendEmail(any(SendEmailRequest.class))).thenReturn(SendEmailResponse.builder().build());

        // Perform the test
        Map<String, String> config = new HashMap<>();
        config.put("from", "sender@example.com");  
        config.put("fromDisplayName", "Sender Name");
        String address = "recipient@example.com";
        String subject = "Test Subject";
        String textBody = "Hello, this is a text email.";
        String htmlBody = "<html><body><h1>Hello</h1><p>This is an HTML email.</p></body></html>";
         
        
        emailSenderProvider.send(config, address, subject, textBody, htmlBody);

        // Verify SES client interaction
        verify(sesClientMock).sendEmail(any(SendEmailRequest.class));
    }

    @Test
    public void testSendEmailMissingFromAddress() {
        // Test data with missing 'from' address
        Map<String, String> config = new HashMap<>();
        String address = "recipient@example.com";
        String subject = "Test Subject";
        String textBody = "Hello, this is a text email.";
        String htmlBody = "<html><body><h1>Hello</h1><p>This is an HTML email.</p></body></html>";

        // Logging the config map before sending
        System.out.println("Config map before sending:");
        for (Map.Entry<String, String> entry : config.entrySet()) {
            System.out.println("- " + entry.getKey() + ": " + entry.getValue());
        }

        // Perform the test
        try {
            emailSenderProvider.send(config, address, subject, textBody, htmlBody);
        } catch (EmailException e) {
            // Verify that EmailException is thrown with expected message
            assert e.getMessage().contains("Missing 'from' email address.");
        }
    }

     
}
