// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.RETURNS_SELF;
import static org.mockito.Mockito.doAnswer;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;
import static sequent.keycloak.scanovate_authenticator.ScanovateResultsTest.SUCCESSFUL_RESULTS;
import static sequent.keycloak.scanovate_authenticator.ScanovateResultsTest.json;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.net.URI;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.mockito.ArgumentCaptor;
import org.mockito.MockedStatic;
import sequent.keycloak.voter_enrollment.Utils;

class ScanovateAuthenticatorTest {
  private static final String ACTION_URL =
      "https://kc/realms/r/login-actions/authenticate?session_code=c&execution=e";
  private static final String FLOW_URL = "https://btrust.example.com/flow?process_id=proc-1";
  private static final String STATUS_NOTE = "sequent.read-only.id-card-number-validated";

  private final Map<String, String> config = new HashMap<>();
  private final Map<String, String> authNotes = new HashMap<>();
  private final MultivaluedHashMap<String, String> queryParams = new MultivaluedHashMap<>();
  private final MultivaluedHashMap<String, String> formParams = new MultivaluedHashMap<>();

  private AuthenticationFlowContext context;
  private LoginFormsProvider form;
  private AuthenticationExecutionModel execution;
  private ScanovateClient client;
  private ScanovateAuthenticator authenticator;
  private MockedStatic<Utils> utils;

  @BeforeEach
  void setUp() throws IOException {
    config.put(ScanovateAuthenticatorFactory.FLOW_ID, "3659");
    config.put(ScanovateAuthenticatorFactory.LINK_PARAMS, "{\"country\": \"country\"}");
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE,
        """
        {"default": [
          {"type": "minValue", "minValue": "0.5", "process": "biometric_match",
           "attributePath": "/score", "errorMsg": "scanovateScoringError"}
        ]}
        """);
    config.put(
        ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE,
        """
        {"default": [
          {"UserAttribute": "firstName", "process": "ocr", "attributePath": "/firstName",
           "type": "text"}
        ]}
        """);
    authNotes.put("country", "Spain");
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID, "123456789");

    context = mock(AuthenticationFlowContext.class);
    AuthenticatorConfigModel configModel = mock(AuthenticatorConfigModel.class);
    when(configModel.getConfig()).thenReturn(config);
    when(context.getAuthenticatorConfig()).thenReturn(configModel);

    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> authNotes.get(i.getArgument(0)));
    doAnswer(i -> authNotes.put(i.getArgument(0), i.getArgument(1)))
        .when(authSession)
        .setAuthNote(anyString(), anyString());
    doAnswer(i -> authNotes.remove(i.getArgument(0))).when(authSession).removeAuthNote(anyString());
    RootAuthenticationSessionModel rootSession = mock(RootAuthenticationSessionModel.class);
    when(rootSession.getId()).thenReturn("root-session");
    when(authSession.getParentSession()).thenReturn(rootSession);
    when(context.getAuthenticationSession()).thenReturn(authSession);

    when(context.getEvent()).thenReturn(mock(EventBuilder.class, RETURNS_SELF));
    KeycloakUriInfo uriInfo = mock(KeycloakUriInfo.class);
    when(uriInfo.getQueryParameters()).thenReturn(queryParams);
    when(context.getUriInfo()).thenReturn(uriInfo);
    HttpRequest httpRequest = mock(HttpRequest.class);
    when(httpRequest.getDecodedFormParameters()).thenReturn(formParams);
    when(context.getHttpRequest()).thenReturn(httpRequest);
    when(context.generateAccessCode()).thenReturn("c");
    when(context.getActionUrl("c")).thenReturn(URI.create(ACTION_URL));

    form = mock(LoginFormsProvider.class, RETURNS_SELF);
    when(form.createForm(anyString())).thenReturn(Response.ok().build());
    when(context.form()).thenReturn(form);

    execution = mock(AuthenticationExecutionModel.class);
    when(context.getExecution()).thenReturn(execution);

    client = mock(ScanovateClient.class);
    when(client.fetchAccessToken()).thenReturn("jwt");
    when(client.createSessionLink(eq("jwt"), any()))
        .thenReturn(new SessionLink(FLOW_URL, "proc-1"));
    when(client.fetchResultsForProcess("proc-1")).thenReturn(json(SUCCESSFUL_RESULTS));
    authenticator = new ScanovateAuthenticator(ignored -> client);

    utils = mockStatic(Utils.class);
  }

  @AfterEach
  void tearDown() {
    utils.close();
  }

  @Test
  void alreadyVerifiedUserSkipsVerification() throws IOException {
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute(STATUS_NOTE)).thenReturn("VERIFIED");
    when(context.getUser()).thenReturn(user);

    authenticator.authenticate(context);

    verify(context).success();
    verify(client, never()).fetchAccessToken();
  }

  @Test
  void authenticateRedirectsToBTrustFlow() throws IOException {
    authenticator.authenticate(context);

    ArgumentCaptor<LinkRequest> request = ArgumentCaptor.forClass(LinkRequest.class);
    verify(client).createSessionLink(eq("jwt"), request.capture());
    assertEquals(3659, request.getValue().flowId());
    assertEquals(ACTION_URL, request.getValue().redirectUrl());
    assertEquals("123456789", request.getValue().idNumber());
    assertEquals(Map.of("country", "Spain"), request.getValue().params());
    assertEquals(SaveOption.DEFAULT, request.getValue().saveOption());
    assertEquals("proc-1", authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));

    ArgumentCaptor<Response> challenge = ArgumentCaptor.forClass(Response.class);
    verify(context).challenge(challenge.capture());
    assertEquals(303, challenge.getValue().getStatus());
    assertEquals(URI.create(FLOW_URL), challenge.getValue().getLocation());
  }

  @Test
  void returnFromBTrustShowsConfirmation() {
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    assertEquals("JUAN", authNotes.get("firstName"));
    assertEquals("VERIFIED", authNotes.get(STATUS_NOTE));
    assertNull(authNotes.get(ScanovateAuthenticator.PROCESS_ID_NOTE));
    verify(form)
        .setAttribute(
            ScanovateAuthenticator.FTL_STORED_ATTRIBUTES,
            List.of(new StoredAttribute("firstName", "JUAN", "text")));
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
    verify(context, never()).success();
  }

  @Test
  void returnHandledAsRefreshIsProcessedInAuthenticate() throws IOException {
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    queryParams.add(ScanovateAuthenticator.PROCESS_ID_QUERY_PARAM, "proc-1");

    authenticator.authenticate(context);

    verify(client).fetchResultsForProcess("proc-1");
    verify(client, never()).createSessionLink(any(), any());
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
  }

  @Test
  void unknownProcessIdInQueryStartsANewSession() throws IOException {
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    queryParams.add(ScanovateAuthenticator.PROCESS_ID_QUERY_PARAM, "someone-else");

    authenticator.authenticate(context);

    verify(client, never()).fetchResultsForProcess(anyString());
    verify(client).createSessionLink(eq("jwt"), any());
  }

  @Test
  void returnWithoutStoredAttributesSucceedsDirectly() {
    config.remove(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_STORE);
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    verify(context).success();
    assertEquals("VERIFIED", authNotes.get(STATUS_NOTE));
  }

  @Test
  void confirmationCompletesTheVerification() {
    authNotes.put(STATUS_NOTE, "VERIFIED");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CONFIRM.value());

    authenticator.action(context);

    verify(context).success();
  }

  @Test
  void forgedConfirmationDoesNotSucceed() throws IOException {
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.CONFIRM.value());

    authenticator.action(context);

    verify(context, never()).success();
    verify(client).createSessionLink(eq("jwt"), any());
  }

  @Test
  void failedRuleShowsRetryableError() throws IOException {
    when(client.fetchResultsForProcess("proc-1"))
        .thenReturn(
            json(
                "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": true, \"errorCode\": 0,"
                    + " \"resultsList\": [{\"process\": \"biometric_match\", \"success\": true,"
                    + " \"score\": 0.1}]}}"));
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    assertEquals("1", authNotes.get(ScanovateAuthenticator.ATTEMPTS_NOTE));
    assertNull(authNotes.get(STATUS_NOTE));
    verify(form).setAttribute(ScanovateAuthenticator.FTL_ERROR, "scanovateScoringError");
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
    verify(form).createForm(ScanovateAuthenticator.ERROR_FORM);
    verify(context, never()).success();
  }

  @Test
  void flowFailureIsMappedToItsMessage() throws IOException {
    when(client.fetchResultsForProcess("proc-1"))
        .thenReturn(
            json(
                "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": false, \"errorCode\": 1026}}"));
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    verify(form)
        .setAttribute(
            ScanovateAuthenticator.FTL_ERROR, ScanovateError.DOCUMENT_AUTHENTICATION.messageKey());
  }

  @Test
  void lastAllowedAttemptRejectsTheVoter() throws IOException {
    when(client.fetchResultsForProcess("proc-1"))
        .thenReturn(
            json(
                "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": false, \"errorCode\": 1030}}"));
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "2");

    authenticator.action(context);

    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.MAX_RETRIES.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
  }

  @Test
  void retryStartsANewSession() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "1");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.RETRY.value());

    authenticator.action(context);

    verify(client).createSessionLink(eq("jwt"), any());
  }

  @Test
  void retryAfterMaxAttemptsIsRejected() throws IOException {
    authNotes.put(ScanovateAuthenticator.ATTEMPTS_NOTE, "3");
    formParams.add(ScanovateAuthenticator.FORM_ACTION_PARAM, FormAction.RETRY.value());

    authenticator.action(context);

    verify(client, never()).createSessionLink(any(), any());
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.MAX_RETRIES.messageKey());
  }

  @Test
  void autoCompleteModeFetchesResultsWithoutRedirect() throws IOException {
    config.put(ScanovateAuthenticatorFactory.EXECUTION_MODE, ExecutionMode.AUTO_COMPLETE.value());

    authenticator.authenticate(context);

    verify(client).fetchResultsForProcess("proc-1");
    verify(form).createForm(ScanovateAuthenticator.CONFIRMATION_FORM);
  }

  @Test
  void missingFlowIdIsAnInternalError() throws IOException {
    config.remove(ScanovateAuthenticatorFactory.FLOW_ID);

    authenticator.authenticate(context);

    verify(client, never()).fetchAccessToken();
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, false);
  }

  @Test
  void apiFailureIsARetryableInternalError() throws IOException {
    when(client.fetchAccessToken()).thenThrow(new IOException("down"));

    authenticator.authenticate(context);

    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
    verify(form).setAttribute(ScanovateAuthenticator.FTL_CAN_RETRY, true);
  }

  @Test
  void documentTypeWithoutRulesIsRejected() {
    config.put(ScanovateAuthenticatorFactory.ATTRIBUTES_TO_VALIDATE, "{\"passport\": []}");
    authNotes.put(ScanovateAuthenticatorFactory.DEFAULT_DOC_ID_TYPE, "seamanBook");
    authNotes.put(ScanovateAuthenticator.PROCESS_ID_NOTE, "proc-1");

    authenticator.action(context);

    verify(context, never()).success();
    assertNull(authNotes.get(STATUS_NOTE));
    verify(form)
        .setAttribute(ScanovateAuthenticator.FTL_ERROR, ScanovateError.INTERNAL.messageKey());
  }

  @Test
  void alternativeExecutionFallsThroughOnError() throws IOException {
    when(execution.isAlternative()).thenReturn(true);
    when(client.fetchAccessToken()).thenThrow(new IOException("down"));

    authenticator.authenticate(context);

    verify(context).attempted();
    verify(form, never()).createForm(anyString());
  }

  @Test
  void linkParamsSkipsMissingAuthNotes() throws ScanovateException {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    config.put(
        ScanovateAuthenticatorFactory.LINK_PARAMS,
        "{\"country\": \"country\", \"x\": \"missing\"}");
    assertEquals(
        Map.of("country", "Spain"), ScanovateAuthenticator.linkParams(config, authSession));
  }

  @Test
  void linkParamsRejectsNonObject() {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    config.put(ScanovateAuthenticatorFactory.LINK_PARAMS, "[]");
    assertThrows(
        ScanovateException.class, () -> ScanovateAuthenticator.linkParams(config, authSession));
  }
}
