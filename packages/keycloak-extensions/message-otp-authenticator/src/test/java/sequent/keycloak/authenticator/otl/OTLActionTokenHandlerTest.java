// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.otl;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.common.VerificationException;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.util.JsonSerialization;
import sequent.keycloak.authenticator.Utils;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;

/**
 * A one-time link verifies only the send it came from: once the voter goes back and the link's code
 * is forgotten, or a newer link replaces it, the old link is refused, so it can't verify a contact
 * entered after it was sent.
 */
class OTLActionTokenHandlerTest {
  private final AuthenticationSessionModel original = mock(AuthenticationSessionModel.class);
  private final Map<String, String> notes = new HashMap<>();
  private final OTLActionTokenHandler handler =
      new OTLActionTokenHandler() {
        @Override
        protected AuthenticationSessionModel getOriginalSession(
            ActionTokenContext<OTLActionToken> tokenContext, OTLActionToken token) {
          return original;
        }
      };

  @SuppressWarnings("unchecked")
  private final ActionTokenContext<OTLActionToken> tokenContext = mock(ActionTokenContext.class);

  @BeforeEach
  void setup() {
    when(original.getAuthNote(anyString())).thenAnswer(i -> notes.get(i.getArgument(0)));
  }

  /** The original session right after a link was sent to the voter's contact. */
  private void linkSent(String codeId) {
    notes.put(MessagingAttributes.NOTE_CODE_ID, codeId);
    notes.put(Utils.OTL_VISITED, "false");
  }

  private static OTLActionToken token(String codeId) {
    return new OTLActionToken(
        "voter-1",
        Integer.MAX_VALUE,
        "compound-session",
        new String[0],
        true,
        "https://login.example/",
        "voting-portal",
        codeId);
  }

  private void verifyToken(OTLActionToken token) throws VerificationException {
    for (var verifier : handler.getVerifiers(tokenContext)) {
      verifier.test(token);
    }
  }

  @Test
  void theLiveLinkIsAccepted() throws Exception {
    linkSent("code-1");
    assertDoesNotThrow(() -> verifyToken(token("code-1")));
  }

  /** Back to the details form forgets the link: opening it afterwards verifies nothing. */
  @Test
  void aLinkForgottenByGoingBackIsRefused() {
    linkSent("code-1");
    notes.remove(Utils.OTL_VISITED);
    assertThrows(VerificationException.class, () -> verifyToken(token("code-1")));
  }

  /** A new link, to a changed contact, replaces the old one. */
  @Test
  void aLinkReplacedByANewerSendIsRefused() {
    linkSent("code-2");
    assertThrows(VerificationException.class, () -> verifyToken(token("code-1")));
  }

  @Test
  void aLinkWithoutItsSendIsRefused() throws Exception {
    linkSent("code-1");
    String legacy =
        JsonSerialization.writeValueAsString(token(null)).replace(",\"code-id\":null", "");
    OTLActionToken read = JsonSerialization.readValue(legacy, OTLActionToken.class);
    assertNull(read.getCodeId());
    assertThrows(VerificationException.class, () -> verifyToken(read));
  }

  @Test
  void theSendTravelsWithTheToken() throws Exception {
    OTLActionToken read =
        JsonSerialization.readValue(
            JsonSerialization.writeValueAsString(token("code-1")), OTLActionToken.class);
    assertEquals("code-1", read.getCodeId());
  }
}
