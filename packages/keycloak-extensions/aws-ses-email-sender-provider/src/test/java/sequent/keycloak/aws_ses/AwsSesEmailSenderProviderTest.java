// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.aws_ses;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

import jakarta.mail.internet.InternetAddress;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.email.EmailException;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import software.amazon.awssdk.services.ses.SesClient;
import software.amazon.awssdk.services.ses.model.SendEmailRequest;
import software.amazon.awssdk.services.ses.model.SendEmailResponse;

@ExtendWith(MockitoExtension.class)
public class AwsSesEmailSenderProviderTest {

  private SesClient sesClientMock;

  private AwsSesEmailSenderProvider emailSenderProvider;

  public AwsSesEmailSenderProviderTest(@Mock SesClient sesClientMock) {
    this.sesClientMock = sesClientMock;
    this.emailSenderProvider = new AwsSesEmailSenderProvider(sesClientMock);
  }

  @Test
  public void testSendEmail() throws EmailException {
    SendEmailResponse mockResponse = SendEmailResponse.builder().messageId("mockMessageId").build();
    when(sesClientMock.sendEmail(any(SendEmailRequest.class))).thenReturn(mockResponse);

    Map<String, String> config = setupValidConfig("sender@example.com", "Sender Name");
    String address = "recipient@example.com";
    String subject = "Test Subject";
    String textBody = "Hello, this is a text email.";
    String htmlBody = "<html><body><h1>Hello</h1><p>This is an HTML email.</p></body></html>";

    emailSenderProvider.send(config, address, subject, textBody, htmlBody);

    verify(sesClientMock, times(1)).sendEmail(any(SendEmailRequest.class));
  }

  @Test
  void testValidToInternetAddress() throws Exception {
    String email = "sender@eample.com";
    String displayName = "Sender Name";
    InternetAddress internetAddress = emailSenderProvider.toInternetAddress(email, displayName);
    assertEquals(displayName, internetAddress.getPersonal());
    assertEquals(email, internetAddress.getAddress());
  }

  @Test
  void testInvalidToInternetAddress() {
    String email = "";
    String displayName = "Sender Name";
    assertThrows(
        EmailException.class, () -> emailSenderProvider.toInternetAddress(email, displayName));
  }

  @Test
  public void testSendEmailGeneralException() {
    Map<String, String> config = setupValidConfig("sender@eample.com", "Sender Name");
    String address = "recipient@example.com";
    String subject = "Test Subject";
    String textBody = "Hello, this is a text email.";
    String htmlBody = "<html><body><h1>Hello</h1><p>This is an HTML email.</p></body></html>";

    doThrow(new RuntimeException("General error"))
        .when(sesClientMock)
        .sendEmail(any(SendEmailRequest.class));

    EmailException thrown =
        assertThrows(
            EmailException.class,
            () -> {
              emailSenderProvider.send(config, address, subject, textBody, htmlBody);
            });

    assertEquals("Exception: Failed to send email via AWS SES", thrown.getMessage());
  }

  public Map<String, String> setupValidConfig(String from, String fromDisplayName) {
    Map<String, String> config = new HashMap<>();
    config.put("from", from);
    config.put("fromDisplayName", fromDisplayName);
    return config;
  }
}
