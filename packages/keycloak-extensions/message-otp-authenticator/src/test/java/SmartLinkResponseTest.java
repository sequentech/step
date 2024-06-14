// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

public class SmartLinkResponseTest {

    @Test
    public void testSmartLinkResponse() throws Exception {
        SmartLinkResponse response = new SmartLinkResponse();
        response.setUserId("123");
        response.setLink("http://example.com/link");
        response.setSent(true);

        assertEquals("123", response.getUserId());
        assertEquals("http://example.com/link", response.getLink());
        assertEquals(true, response.isSent());

        // Testing JSON serialization/deserialization using ObjectMapper
        ObjectMapper mapper = new ObjectMapper();
        String json = mapper.writeValueAsString(response);

        SmartLinkResponse deserialized = mapper.readValue(json, SmartLinkResponse.class);
        assertEquals(response.getUserId(), deserialized.getUserId());
        assertEquals(response.getLink(), deserialized.getLink());
        assertEquals(response.isSent(), deserialized.isSent());
    }
}

