// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import software.amazon.awssdk.services.sns.SnsClient;
import software.amazon.awssdk.services.sns.model.MessageAttributeValue;
import software.amazon.awssdk.services.sns.model.PublishResponse;
import software.amazon.awssdk.services.sns.model.SnsException;

@JBossLog
public class AwsSmsSenderProvider implements SmsSenderProvider {

  private static final SnsClient sns = SnsClient.create();
  private final String senderId;

  AwsSmsSenderProvider(String senderId) {
    this.senderId = senderId;
  }

  @Override
  public void send(String phoneNumber, String message) throws IOException {
    log.infov("**Sending AWS SMS**:\n\t- phoneNumber={0}\n\t- message={1}", phoneNumber, message);
    Map<String, MessageAttributeValue> messageAttributes = new HashMap<>();
    messageAttributes.put(
        "AWS.SNS.SMS.SenderID",
        MessageAttributeValue.builder().stringValue(senderId).dataType("String").build());
    messageAttributes.put(
        "AWS.SNS.SMS.SMSType",
        MessageAttributeValue.builder().stringValue("Transactional").dataType("String").build());

    log.infov("AWS_ENDPOINT_URL: {0}", System.getenv("AWS_ENDPOINT_URL"));
    log.infov("AWS_ENDPOINT_URL_SNS: {0}", System.getenv("AWS_ENDPOINT_URL_SNS"));

    try {
      PublishResponse result =
          sns.publish(
              builder ->
                  builder
                      .message(message)
                      .phoneNumber(phoneNumber)
                      .messageAttributes(messageAttributes));
      log.infov(
          result.messageId() + " Message sent. Status is " + result.sdkHttpResponse().statusCode());
    } catch (SnsException e) {
      log.infov(e.awsErrorDetails().errorMessage());
      throw new IOException(e.awsErrorDetails().errorMessage());
    }
  }

  @Override
  public void close() {}
}
