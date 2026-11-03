// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.inetum_authenticator;

import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.lang.reflect.Method;
import java.util.Collection;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.UserModel;
import org.mockito.ArgumentMatchers;

class DeferredRegistrationEventDetailsTest {

  /**
   * The event listener classifies events by these details, so they must come only from the
   * authenticators that set them, while ordinary form fields stay in the registration event.
   */
  @Test
  void eventDetailsKeepFormFieldsExceptListenerDetails() throws Exception {
    ValidationContext context = mock(ValidationContext.class);
    EventBuilder event = mock(EventBuilder.class);
    when(context.getEvent()).thenReturn(event);
    List<String> listenerDetails = List.of("type", "msgBody");
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.FIRST_NAME, "Ana");
    listenerDetails.forEach(detail -> formData.add(detail, "communications"));

    Method method =
        DeferredRegistrationUserCreation.class.getDeclaredMethod(
            "buildEventDetails", MultivaluedMap.class, ValidationContext.class, UserModel.class);
    method.setAccessible(true);
    method.invoke(new DeferredRegistrationUserCreation(), formData, context, null);

    verify(event).detail(UserModel.FIRST_NAME, List.of("Ana"));
    listenerDetails.forEach(
        detail -> {
          verify(event, never()).detail(eq(detail), ArgumentMatchers.<Collection<String>>any());
          verify(event, never()).detail(eq(detail), ArgumentMatchers.<String>any());
        });
  }
}
