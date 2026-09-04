// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.util.Map;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import software.amazon.awssdk.services.sns.model.MessageAttributeValue;

class AwsSmsSenderProviderTest {

  @BeforeAll
  static void configureAwsRegion() {
    System.setProperty("aws.region", "eu-west-1");
  }

  @Test
  void usesOriginationNumberWhenConfigured() {
    Map<String, MessageAttributeValue> attributes =
        AwsSmsSenderProvider.buildMessageAttributes("SEQUENT", "+13433160806");

    assertEquals(
        "+13433160806", attributes.get("AWS.MM.SMS.OriginationNumber").stringValue());
    assertFalse(attributes.containsKey("AWS.SNS.SMS.SenderID"));
    assertEquals("Transactional", attributes.get("AWS.SNS.SMS.SMSType").stringValue());
  }

  @Test
  void preservesSenderIdWhenOriginationNumberIsNotConfigured() {
    Map<String, MessageAttributeValue> attributes =
        AwsSmsSenderProvider.buildMessageAttributes("SEQUENT", null);

    assertEquals("SEQUENT", attributes.get("AWS.SNS.SMS.SenderID").stringValue());
    assertFalse(attributes.containsKey("AWS.MM.SMS.OriginationNumber"));
  }

  @Test
  void omitsEmptySenderAttributes() {
    Map<String, MessageAttributeValue> attributes =
        AwsSmsSenderProvider.buildMessageAttributes(" ", " ");

    assertFalse(attributes.containsKey("AWS.SNS.SMS.SenderID"));
    assertFalse(attributes.containsKey("AWS.MM.SMS.OriginationNumber"));
    assertEquals("Transactional", attributes.get("AWS.SNS.SMS.SMSType").stringValue());
  }
}
