// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import java.io.IOException;
import java.text.MessageFormat;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.Provider;
import org.keycloak.theme.Theme;

public interface SmsSenderProvider extends Provider {
  default String send(
      String phoneNumber,
      String messageKey,
      List<String> attributes,
      RealmModel realm,
      UserModel user,
      KeycloakSession session)
      throws IOException {
    Locale locale = session.getContext().resolveLocale(user);

    Theme theme = session.theme().getTheme(Theme.Type.LOGIN);
    Properties messages = theme.getEnhancedMessages(realm, locale);
    String formattedMessage =
        new MessageFormat(messages.getProperty(messageKey, messageKey), locale)
            .format(attributes.toArray());
    send(phoneNumber, formattedMessage);

    return String.format(
        "{\"phoneNumber\": \"%s\", \"message\": \"%s\"}", phoneNumber, formattedMessage);
  }

  default void sendFeedback(
      String phoneNumber,
      boolean success,
      RealmModel realm,
      UserModel user,
      KeycloakSession session)
      throws IOException {}

  default void send(String phoneNumber, String message) throws IOException {}
}
