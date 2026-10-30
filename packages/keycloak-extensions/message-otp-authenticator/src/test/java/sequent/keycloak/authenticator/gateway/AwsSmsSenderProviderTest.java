// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

import java.util.function.Consumer;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.MockedStatic;
import sequent.keycloak.authenticator.CapturedLogs;
import software.amazon.awssdk.awscore.exception.AwsErrorDetails;
import software.amazon.awssdk.http.SdkHttpResponse;
import software.amazon.awssdk.services.sns.SnsClient;
import software.amazon.awssdk.services.sns.model.PublishRequest;
import software.amazon.awssdk.services.sns.model.PublishResponse;
import software.amazon.awssdk.services.sns.model.SnsException;

class AwsSmsSenderProviderTest {
  private static final SnsClient sns = mock(SnsClient.class);

  @BeforeAll
  static void installMockSnsClient() {
    try (MockedStatic<SnsClient> factory = mockStatic(SnsClient.class)) {
      factory.when(SnsClient::create).thenReturn(sns);
      new AwsSmsSenderProvider("SEQUENT");
    }
  }

  @BeforeEach
  void resetSns() {
    reset(sns);
  }

  @Test
  void deliversTheSecretWithoutLoggingTheMessage() throws Exception {
    PublishRequest.Builder request = PublishRequest.builder();
    when(sns.publish(any(Consumer.class)))
        .thenAnswer(
            i -> {
              Consumer<PublishRequest.Builder> configure = i.getArgument(0);
              configure.accept(request);
              return PublishResponse.builder()
                  .messageId("synthetic-message")
                  .sdkHttpResponse(SdkHttpResponse.builder().statusCode(200).build())
                  .build();
            });
    try (CapturedLogs logs = new CapturedLogs(AwsSmsSenderProvider.class)) {
      new AwsSmsSenderProvider("SEQUENT").send("+15550123456", "Your code is 482619");
      assertEquals("Your code is 482619", request.build().message());
      assertFalse(logs.text().contains("482619"));
    }
  }

  @Test
  void providerFailureDoesNotExposeAnEchoedMessage() {
    when(sns.publish(any(Consumer.class)))
        .thenThrow(
            SnsException.builder()
                .statusCode(400)
                .awsErrorDetails(AwsErrorDetails.builder().errorMessage("OTP 482619").build())
                .build());
    try (CapturedLogs logs = new CapturedLogs(AwsSmsSenderProvider.class)) {
      java.io.IOException error =
          assertThrows(
              java.io.IOException.class,
              () -> new AwsSmsSenderProvider("SEQUENT").send("+15550123456", "OTP 482619"));
      assertEquals("AWS SMS delivery failed", error.getMessage());
      assertNull(error.getCause());
      assertFalse(logs.text().contains("482619"));
    }
  }
}
