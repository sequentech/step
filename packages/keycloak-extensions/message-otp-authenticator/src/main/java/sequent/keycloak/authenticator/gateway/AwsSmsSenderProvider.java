// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import java.util.HashMap;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import software.amazon.awssdk.services.sns.SnsClient;
import software.amazon.awssdk.services.sns.model.MessageAttributeValue;

@JBossLog
public class AwsSmsSenderProvider implements SmsSenderProvider {

  private static final SnsClient sns = SnsClient.create();
  private final String senderId;

  AwsSmsSenderProvider(String senderId) {
    this.senderId = senderId;
  }

  @Override
  public void send(String phoneNumber, String message) {
    log.infov("**Sending AWS SMS**:\n\t- phoneNumber={0}\n\t- message={1}", phoneNumber, message);
    Map<String, MessageAttributeValue> messageAttributes = new HashMap<>();
    messageAttributes.put(
        "AWS.SNS.SMS.SenderID",
        MessageAttributeValue.builder().stringValue(senderId).dataType("String").build());
    messageAttributes.put(
        "AWS.SNS.SMS.SMSType",
        MessageAttributeValue.builder().stringValue("Transactional").dataType("String").build());

    sns.publish(
        builder ->
            builder.message(message).phoneNumber(phoneNumber).messageAttributes(messageAttributes));
  }

  @Override
  public void close() {}
}
