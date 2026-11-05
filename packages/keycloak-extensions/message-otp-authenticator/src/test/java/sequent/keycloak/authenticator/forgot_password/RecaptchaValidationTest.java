// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import org.apache.http.client.methods.CloseableHttpResponse;
import org.apache.http.client.methods.HttpPost;
import org.apache.http.entity.StringEntity;
import org.apache.http.impl.client.CloseableHttpClient;
import org.apache.http.util.EntityUtils;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.connections.httpclient.HttpClientProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.mockito.ArgumentCaptor;
import sequent.keycloak.authenticator.CapturedLogs;

class RecaptchaValidationTest {
  private static final String SECRET = "synthetic-site-secret";
  private static final String TOKEN = "synthetic-response-token";
  private final AuthenticationFlowContext context =
      mock(AuthenticationFlowContext.class, RETURNS_DEEP_STUBS);
  private final CloseableHttpClient client = mock(CloseableHttpClient.class);

  @BeforeEach
  void setup() {
    KeycloakSession session = mock(KeycloakSession.class);
    HttpClientProvider provider = mock(HttpClientProvider.class);
    when(context.getSession()).thenReturn(session);
    when(session.getProvider(HttpClientProvider.class)).thenReturn(provider);
    when(provider.getHttpClient()).thenReturn(client);
    when(context.getConnection().getRemoteAddr()).thenReturn("192.0.2.1");
  }

  private void response(String body) throws Exception {
    CloseableHttpResponse response = mock(CloseableHttpResponse.class);
    when(response.getEntity()).thenReturn(new StringEntity(body, StandardCharsets.UTF_8));
    when(client.execute(any(HttpPost.class))).thenReturn(response);
  }

  @Test
  void successfulVerificationSendsCredentialsOnlyToTheProvider() throws Exception {
    response("{\"success\":true,\"score\":0.9,\"echo\":\"" + TOKEN + "\"}");
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      assertTrue(Utils.validateRecaptcha(context, false, TOKEN, SECRET, 0.5));
      ArgumentCaptor<HttpPost> request = ArgumentCaptor.forClass(HttpPost.class);
      verify(client).execute(request.capture());
      assertEquals(Utils.RECAPTCHA_SITE_VERIFY_URL, request.getValue().getURI().toString());
      assertEquals(
          "secret=synthetic-site-secret&response=synthetic-response-token&remoteip=192.0.2.1",
          EntityUtils.toString(request.getValue().getEntity()));
      assertFalse(logs.text().contains(SECRET));
      assertFalse(logs.text().contains(TOKEN));
    }
  }

  @Test
  void lowScoresAndProviderRejectionsAreNotAccepted() throws Exception {
    for (String body :
        List.of(
            "{\"success\":true,\"score\":0.1}",
            "{\"success\":true,\"score\":0.5}",
            "{\"success\":false,\"score\":0.9}",
            "{\"score\":0.9}")) {
      response(body);
      assertFalse(Utils.validateRecaptcha(context, false, TOKEN, SECRET, 0.5));
    }
  }

  @Test
  void malformedResponseFailsClosedWithoutLoggingItsBody() throws Exception {
    response("invalid " + SECRET + " " + TOKEN);
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      assertFalse(Utils.validateRecaptcha(context, true, TOKEN, SECRET, 0.5));
      assertFalse(logs.text().contains(SECRET));
      assertFalse(logs.text().contains(TOKEN));
    }
  }

  @Test
  void transportFailureFailsClosedWithoutLoggingItsExceptionPayload() throws Exception {
    when(client.execute(any(HttpPost.class))).thenThrow(new IOException(SECRET + " " + TOKEN));
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      assertFalse(Utils.validateRecaptcha(context, true, TOKEN, SECRET, 0.5));
      assertFalse(logs.text().contains(SECRET));
      assertFalse(logs.text().contains(TOKEN));
    }
  }

  @Test
  void interruptedResponseIsClosedAndRejectedWithoutLoggingItsPayload() throws Exception {
    java.util.concurrent.atomic.AtomicBoolean closed =
        new java.util.concurrent.atomic.AtomicBoolean();
    java.io.InputStream stream =
        new java.io.InputStream() {
          @Override
          public int read() throws IOException {
            throw new IOException(SECRET + " " + TOKEN);
          }

          @Override
          public void close() {
            closed.set(true);
          }
        };
    org.apache.http.entity.BasicHttpEntity entity = new org.apache.http.entity.BasicHttpEntity();
    entity.setContent(stream);
    CloseableHttpResponse response = mock(CloseableHttpResponse.class);
    when(response.getEntity()).thenReturn(entity);
    when(client.execute(any(HttpPost.class))).thenReturn(response);
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      assertFalse(Utils.validateRecaptcha(context, true, TOKEN, SECRET, 0.5));
      assertTrue(closed.get());
      assertFalse(logs.text().contains(SECRET));
      assertFalse(logs.text().contains(TOKEN));
    }
  }

  @Test
  void optionalConfigurationFallbacksDoNotLogTheSiteSecret() {
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(Map.of(Utils.RECAPTCHA_SITE_SECRET_ATTRIBUTE, SECRET));
    try (CapturedLogs logs = new CapturedLogs(Utils.class)) {
      assertEquals("1", Utils.getString(config, Utils.RECAPTCHA_MIN_SCORE_ATTRIBUTE, "1"));
      assertEquals(10, Utils.getInt(config, Utils.MAX_CANDIDATES, "10"));
      assertFalse(Utils.getBoolean(config, Utils.RECAPTCHA_ENABLED_ATTRIBUTE, false));
      assertEquals(
          List.of("username"),
          Utils.getMultivalueString(config, Utils.USERNAME_ATTRIBUTES, List.of("username")));
      assertFalse(logs.text().contains(SECRET));
    }
  }
}
