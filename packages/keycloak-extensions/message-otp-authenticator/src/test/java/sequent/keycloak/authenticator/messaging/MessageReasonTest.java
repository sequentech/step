// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.*;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

/**
 * R2-22: a refusal has one reason string, shared by Keycloak and harvest. The shared reasons are
 * sequent-core's {@code MessageReason} strings, read from its source.
 */
class MessageReasonTest {
  private static final Path SEQUENT_CORE =
      Path.of("..", "..", "sequent-core", "src", "types", "messaging.rs");

  @Test
  void everySharedReasonIsSequentCoresString() throws Exception {
    String source = Files.readString(SEQUENT_CORE);
    Set<String> rust = new HashSet<>();
    Matcher matcher =
        Pattern.compile("#\\[strum\\(\\s*serialize = \"([^\"]+)\"\\s*\\)\\]").matcher(source);
    while (matcher.find()) {
      rust.add(matcher.group(1));
    }
    assertFalse(rust.isEmpty());
    for (MessageReason reason : MessageReason.values()) {
      if (reason.sharedWithHarvest()) {
        assertTrue(rust.contains(reason.reason()), reason.name() + " differs from sequent-core");
      } else {
        assertFalse(rust.contains(reason.reason()), reason.name() + " is not Keycloak's only");
      }
    }
  }

  @Test
  void codeRefusalsUseTheSharedReasons() {
    assertEquals("No contact channel", CodeChannels.Refusal.NO_CONTACT.reason());
    assertEquals("Channel not offered for codes", CodeChannels.Refusal.NOT_OFFERED.reason());
  }
}
