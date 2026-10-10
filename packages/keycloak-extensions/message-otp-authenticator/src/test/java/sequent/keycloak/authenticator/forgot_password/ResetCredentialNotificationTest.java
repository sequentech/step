// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import java.util.Set;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.browser.AbstractUsernameFormAuthenticator;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.ThemeManager;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.Theme;
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.FakePace;
import sequent.keycloak.authenticator.NeutralPace;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageReason;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.SendMessageResponse;

/**
 * R3-02: a reset link that harvest refused or that failed is recorded as not sent, with its reason;
 * the page is the same for everyone.
 */
class ResetCredentialNotificationTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

  /** The link itself is an action token; the test only needs a stand-in. */
  private final ResetCredentialNotification authenticator =
      new ResetCredentialNotification() {
        @Override
        protected String resetLink(
            AuthenticationFlowContext context, UserModel user, int validityInSecs) {
          return "https://login.example.test/reset?key=synthetic";
        }
      };

  private final AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
  private final AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final UserModel user = mock(UserModel.class);
  private final EventBuilder event = mock(EventBuilder.class, RETURNS_SELF);

  /** The SEND_RESET_PASSWORD event (a clone of the flow's event, which logs communications). */
  private final EventBuilder resetEvent = mock(EventBuilder.class, RETURNS_SELF);

  private final MessageSenderProvider sender = mock(MessageSenderProvider.class);
  private final SmsSenderProvider sms = mock(SmsSenderProvider.class);
  private final Map<String, List<String>> attributes = new HashMap<>();

  @BeforeEach
  void setup() {
    when(context.getUser()).thenReturn(user);
    when(context.getRealm()).thenReturn(realm);
    when(context.getSession()).thenReturn(session);
    when(context.getEvent()).thenReturn(event);
    when(event.clone()).thenReturn(resetEvent);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(realm.getName()).thenReturn("tenant-" + TENANT + "-event-synthetic");
    when(realm.getActionTokenGeneratedByUserLifespan(anyString())).thenReturn(900);
    KeycloakContext keycloakContext = mock(KeycloakContext.class);
    when(session.getContext()).thenReturn(keycloakContext);
    when(keycloakContext.resolveLocale(any())).thenReturn(Locale.ENGLISH);
    ThemeManager themes = mock(ThemeManager.class);
    Theme theme = mock(Theme.class);
    when(session.theme()).thenReturn(themes);
    try {
      when(themes.getTheme(Theme.Type.LOGIN)).thenReturn(theme);
      when(theme.getEnhancedMessages(any(), any())).thenReturn(new Properties());
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(sender);
    when(session.getProvider(SmsSenderProvider.class)).thenReturn(sms);
    when(sender.getChannels())
        .thenReturn(
            Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER));
    when(sender.delivers(any())).thenCallRealMethod();
    when(user.getId()).thenReturn("voter-1");
    when(user.getAttributeStream(anyString()))
        .thenAnswer(i -> attributes.getOrDefault(i.getArgument(0), List.of()).stream());
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("WHATSAPP"));
    attributes.put(MessagingAttributes.MESSAGE_NUMBER, List.of("+15550199999"));
  }

  private void assertNeutralPage() {
    ArgumentCaptor<FormMessage> page = ArgumentCaptor.forClass(FormMessage.class);
    verify(context).forkWithSuccessMessage(page.capture());
    assertEquals("forgotPassword.success.contact.message", page.getValue().getMessage());
  }

  @Test
  void anUnknownAccountGetsTheSamePageAndNothingIsSent() {
    when(context.getUser()).thenReturn(null);
    authenticator.authenticate(context);
    verify(sender, never()).send(any());
    assertNeutralPage();
  }

  @Test
  void aRefusedLinkIsRecordedWithHarvestsReason() {
    when(sender.send(any()))
        .thenReturn(new SendMessageResponse("m-1", "FAILED", "Channel not offered for notices"));
    authenticator.authenticate(context);
    verify(resetEvent).detail("reason", "Channel not offered for notices");
    verify(resetEvent).error(Errors.EMAIL_SEND_FAILED);
    verify(resetEvent, never()).success();
    assertNeutralPage();
  }

  @Test
  void aFailedLinkIsRecordedAsNotSent() {
    when(sender.send(any())).thenReturn(new SendMessageResponse(null, "FAILED", "unreachable"));
    authenticator.authenticate(context);
    verify(resetEvent).detail("reason", "unreachable");
    verify(resetEvent).error(Errors.EMAIL_SEND_FAILED);
    assertNeutralPage();
  }

  @Test
  void aLinkWithoutAChannelIsRecordedAsNotSent() {
    attributes.clear();
    when(sender.send(any()))
        .thenReturn(new SendMessageResponse("m-2", "FAILED", "No contact channel"));
    authenticator.authenticate(context);
    verify(resetEvent).detail("reason", "No contact channel");
    verify(resetEvent).error(Errors.EMAIL_SEND_FAILED);
    assertNeutralPage();
  }

  @Test
  void aLinkHandedOverIsRecordedAsSent() {
    for (String state : List.of("ACCEPTED", "QUEUED", "UNKNOWN")) {
      reset(resetEvent);
      when(sender.send(any())).thenReturn(new SendMessageResponse("m-3", state, null));
      authenticator.authenticate(context);
      verify(resetEvent).success();
      verify(resetEvent, never()).error(anyString());
    }
  }

  /**
   * R7-06: the page answers no sooner than the floor, for an account (whose send takes time) and
   * for no account alike.
   */
  @Test
  void anUnknownAccountWaitsAsLongAsAnAccount() {
    try (FakePace pace = new FakePace()) {
      when(sender.send(any()))
          .thenAnswer(
              i -> {
                pace.advance(900);
                return new SendMessageResponse("m-1", "ACCEPTED", null);
              });
      long start = pace.now();
      authenticator.authenticate(context);
      long withAccount = pace.now() - start;

      when(context.getUser()).thenReturn(null);
      start = pace.now();
      authenticator.authenticate(context);
      long withoutAccount = pace.now() - start;

      assertEquals(NeutralPace.DEFAULT_MILLIS, withAccount);
      assertEquals(withAccount, withoutAccount);
      assertEquals(
          List.of(NeutralPace.DEFAULT_MILLIS - 900, NeutralPace.DEFAULT_MILLIS), pace.held());
    }
  }

  /**
   * R7-08: an SMS provider that sends only codes can't carry a reset link: recorded as not sent.
   */
  @Test
  void anSmsProviderThatSendsOnlyCodesDoesNotCountTheLinkAsSent() throws Exception {
    attributes.put(MessagingAttributes.MESSAGE_CHANNEL, List.of("SMS"));
    attributes.put(MessagingAttributes.MESSAGE_NUMBER, List.of("+15550100777"));
    when(sms.sendsOnlyCodes()).thenReturn(true);

    authenticator.authenticate(context);

    verify(sms, never()).send(anyString(), anyString(), anyList(), any(), any(), any());
    verify(resetEvent).detail("reason", MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason());
    verify(resetEvent).error(Errors.EMAIL_SEND_FAILED);
    verify(resetEvent, never()).success();
    assertNeutralPage();
  }

  /** R7-13: the identifier the visitor typed never reaches the event. */
  @Test
  void theTypedIdentifierNeverReachesTheEvent() {
    when(authSession.getAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME))
        .thenReturn("+15550199999");
    when(sender.send(any())).thenReturn(new SendMessageResponse("m-4", "ACCEPTED", null));
    authenticator.authenticate(context);
    verify(resetEvent, never()).detail(eq(Details.USERNAME), anyString());
    verify(resetEvent, never()).detail(anyString(), eq("+15550199999"));
  }
}
