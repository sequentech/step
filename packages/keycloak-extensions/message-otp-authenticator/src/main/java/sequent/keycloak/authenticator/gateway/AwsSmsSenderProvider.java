// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
  private final String originationNumber;

  AwsSmsSenderProvider(String senderId, String originationNumber) {
    this.senderId = senderId;
    this.originationNumber = originationNumber;
  }

  static Map<String, MessageAttributeValue> buildMessageAttributes(
      String senderId, String originationNumber) {
    Map<String, MessageAttributeValue> messageAttributes = new HashMap<>();
    if (originationNumber != null && !originationNumber.isBlank()) {
      messageAttributes.put(
          "AWS.MM.SMS.OriginationNumber",
          MessageAttributeValue.builder()
              .stringValue(originationNumber)
              .dataType("String")
              .build());
    } else if (senderId != null && !senderId.isBlank()) {
      messageAttributes.put(
          "AWS.SNS.SMS.SenderID",
          MessageAttributeValue.builder().stringValue(senderId).dataType("String").build());
    }
    messageAttributes.put(
        "AWS.SNS.SMS.SMSType",
        MessageAttributeValue.builder().stringValue("Transactional").dataType("String").build());
    return messageAttributes;
  }

  @Override
  public void send(String phoneNumber, String message) throws IOException {
    log.infov("Sending AWS SMS to {0}", phoneNumber);
    Map<String, MessageAttributeValue> messageAttributes =
        buildMessageAttributes(senderId, originationNumber);

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
      log.errorf("AWS SMS delivery failed (status %d)", e.statusCode());
      throw new IOException("AWS SMS delivery failed");
    }
  }

  @Override
  public void close() {}
}
