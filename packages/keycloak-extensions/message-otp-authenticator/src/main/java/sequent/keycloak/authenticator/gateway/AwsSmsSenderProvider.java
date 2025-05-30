// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import software.amazon.awssdk.auth.credentials.AwsCredentialsProvider;
import software.amazon.awssdk.auth.credentials.AwsSessionCredentials;
import software.amazon.awssdk.auth.credentials.StaticCredentialsProvider;
import software.amazon.awssdk.services.sns.SnsClient;
import software.amazon.awssdk.services.sns.model.MessageAttributeValue;
import software.amazon.awssdk.services.sns.model.PublishResponse;
import software.amazon.awssdk.services.sns.model.SnsException;
import software.amazon.awssdk.services.sts.StsClient;
import software.amazon.awssdk.services.sts.model.AssumeRoleRequest;
import software.amazon.awssdk.services.sts.model.AssumeRoleResponse;
import software.amazon.awssdk.services.sts.model.Credentials;
import software.amazon.awssdk.services.sts.model.StsException;

@JBossLog
public class AwsSmsSenderProvider implements SmsSenderProvider {

  private final SnsClient sns;
  private final String senderId;
  private final String roleArn;
  private final String sessionName;

  AwsSmsSenderProvider(String senderId) {
    this(senderId, null, "AwsSmsSenderSession");
  }

  AwsSmsSenderProvider(String senderId, String roleArn, String sessionName) {
    this.senderId = senderId;
    this.roleArn = roleArn;
    this.sessionName = sessionName;
    this.sns = createSnsClient();
  }

  private SnsClient createSnsClient() {
    if (roleArn != null && !roleArn.trim().isEmpty()) {
      log.infov("Creating SNS client with assumed role: {0}", roleArn);
      return SnsClient.builder()
          .credentialsProvider(createAssumedRoleCredentialsProvider())
          .build();
    } else {
      log.info("Creating SNS client with default credentials");
      return SnsClient.create();
    }
  }

  private AwsCredentialsProvider createAssumedRoleCredentialsProvider() {
    try (StsClient stsClient = StsClient.create()) {
      AssumeRoleRequest assumeRoleRequest = AssumeRoleRequest.builder()
          .roleArn(roleArn)
          .roleSessionName(sessionName)
          .durationSeconds(3600) // 1 hour session
          .build();

      AssumeRoleResponse assumeRoleResponse = stsClient.assumeRole(assumeRoleRequest);
      Credentials credentials = assumeRoleResponse.credentials();

      AwsSessionCredentials awsCredentials = AwsSessionCredentials.create(
          credentials.accessKeyId(),
          credentials.secretAccessKey(),
          credentials.sessionToken()
      );

      return StaticCredentialsProvider.create(awsCredentials);
    } catch (StsException e) {
      log.errorv("Failed to assume role {0}: {1}", roleArn, e.awsErrorDetails().errorMessage());
      throw new RuntimeException("Failed to assume role: " + e.awsErrorDetails().errorMessage(), e);
    }
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
  public void close() {
    if (sns != null) {
      sns.close();
    }
  }
}
