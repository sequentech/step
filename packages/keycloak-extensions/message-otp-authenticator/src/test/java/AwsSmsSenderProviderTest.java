// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/* 
package sequent.keycloak.authenticator.gateway;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import software.amazon.awssdk.services.sns.SnsClient;
import software.amazon.awssdk.services.sns.model.PublishRequest;
import software.amazon.awssdk.services.sns.model.PublishResponse;
import software.amazon.awssdk.services.sns.model.SnsException;
import software.amazon.awssdk.services.sns.model.MessageAttributeValue;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

public class AwsSmsSenderProviderTest {

    private AwsSmsSenderProvider smsSender;
    private SnsClient snsClientMock;

    @BeforeEach
    public void setUp() {
        snsClientMock = mock(SnsClient.class);
        smsSender = new AwsSmsSenderProvider("TestSenderId");
        // Inject the mock SnsClient into the AwsSmsSenderProvider
        // This can be done using reflection or using a setter method if it's exposed
        // Example of reflection: setField(smsSender, "sns", snsClientMock);
    }

    @AfterEach
    public void tearDown() {
        // Clean up resources if needed
        smsSender.close();
    }

    @Test
    public void testSend() {
        // Prepare test data
        String phoneNumber = "+1234567890";
        String message = "Test SMS message";

        // Mock behavior of SnsClient.publish method
        PublishResponse publishResponse = PublishResponse.builder().messageId("testMessageId").build();
        when(snsClientMock.publish(any(PublishRequest.class))).thenReturn(publishResponse);

        // Invoke the method under test
        smsSender.send(phoneNumber, message);

        // Verify that SnsClient.publish was called with correct parameters
        verify(snsClientMock).publish(argThat(request ->
                request.message().equals(message) &&
                request.phoneNumber().equals(phoneNumber) &&
                request.messageAttributes().get("AWS.SNS.SMS.SenderID").stringValue().equals("TestSenderId") &&
                request.messageAttributes().get("AWS.SNS.SMS.SMSType").stringValue().equals("Transactional")
        ));
    }

    @Test
    public void testSendExceptionHandling() {
        // Prepare test data
        String phoneNumber = "+1234567890";
        String message = "Test SMS message";

        // Mock behavior of SnsClient.publish method to throw an exception
        when(snsClientMock.publish(any(PublishRequest.class))).thenThrow(SnsException.builder().message("AWS SNS exception").build());

        // Invoke the method under test
        assertThrows(SnsException.class, () -> smsSender.send(phoneNumber, message));

        // Optionally, verify that no interaction with SnsClient happened after the exception
        verifyNoMoreInteractions(snsClientMock);
    }
}

*/