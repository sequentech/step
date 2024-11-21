// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.email.EmailException;
import sequent.keycloak.dummy.DummyEmailSenderProvider;

public class DummyEmailSenderProviderTest {

  private DummyEmailSenderProvider emailSender;

  @BeforeEach
  public void setUp() {
    emailSender = new DummyEmailSenderProvider();
  }

  @Test
  public void testSend() throws EmailException {

    Map<String, String> config = new HashMap<>();
    String address = "test@example.com";
    String subject = "Test Subject";
    String textBody = "This is the text body";
    String htmlBody = "<html><body><p>This is the HTML body</p></body></html>";

    emailSender.send(config, address, subject, textBody, htmlBody);
  }
}
