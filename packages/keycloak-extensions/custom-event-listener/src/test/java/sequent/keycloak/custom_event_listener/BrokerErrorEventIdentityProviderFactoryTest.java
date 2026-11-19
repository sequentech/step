// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.Response;
import org.junit.jupiter.api.Test;
import org.keycloak.broker.provider.UserAuthenticationIdentityProvider;
import org.keycloak.events.Details;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.models.IdentityProviderModel;

class BrokerErrorEventIdentityProviderFactoryTest {

  private static EventBuilder failureBuilder(EventBuilder event) {
    var failure = mock(EventBuilder.class);
    when(event.clone()).thenReturn(failure);
    when(failure.event(any())).thenReturn(failure);
    when(failure.detail(anyString(), anyString())).thenReturn(failure);
    return failure;
  }

  @Test
  void anAccessDeniedReturnIsRecordedBeforeKeycloakHandlesIt() {
    var event = mock(EventBuilder.class);
    var failure = failureBuilder(event);
    var delegate = mock(UserAuthenticationIdentityProvider.AuthenticationCallback.class);
    var idp = new IdentityProviderModel();
    idp.setAlias("external-login");
    var response = Response.ok().build();
    when(delegate.cancelled(idp)).thenReturn(response);

    var callback = new BrokerErrorEventIdentityProviderFactory.ErrorEventCallback(delegate, event);
    assertSame(response, callback.cancelled(idp));

    var order = inOrder(failure, delegate);
    order.verify(failure).event(EventType.IDENTITY_PROVIDER_LOGIN);
    order.verify(failure).detail(Details.IDENTITY_PROVIDER, "external-login");
    order.verify(failure).error("access_denied");
    order.verify(delegate).cancelled(idp);
    // The broker's own builder is left as it was.
    verify(event, never()).error(anyString());
  }

  @Test
  void anyOtherErrorReturnIsRecordedWithItsReason() {
    var event = mock(EventBuilder.class);
    var failure = failureBuilder(event);
    var delegate = mock(UserAuthenticationIdentityProvider.AuthenticationCallback.class);
    var idp = new IdentityProviderModel();
    idp.setAlias("external-login");

    new BrokerErrorEventIdentityProviderFactory.ErrorEventCallback(delegate, event)
        .error(idp, "login_required");

    verify(failure).detail(Details.REASON, "login_required");
    verify(failure).error("identity_provider_error");
    verify(delegate).error(idp, "login_required");
  }

  @Test
  void theOverridingFactoryKeepsTheBuiltInProviderId() {
    assertEquals("oidc", new BrokerErrorEventIdentityProviderFactory().getId());
  }
}
