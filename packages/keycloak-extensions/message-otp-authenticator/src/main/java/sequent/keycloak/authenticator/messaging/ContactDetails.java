// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.Locale;
import java.util.regex.Pattern;
import lombok.experimental.UtilityClass;

/**
 * Keeps voters' contacts out of Keycloak events, which reach the electoral log: no mobile number,
 * email address or messaging account, whatever attribute holds it.
 */
@UtilityClass
public class ContactDetails {
  private final Pattern NUMBER = Pattern.compile("\\+[1-9][0-9]{6,14}");
  private final Pattern EMAIL = Pattern.compile("[^\\s\"'<>@]+@[^\\s\"'<>@]+\\.[A-Za-z]{2,}");

  /**
   * Whether an attribute or note may hold a contact: the messaging attributes, the email, any
   * mobile or phone attribute (the configured SMS attribute among them), or a value that reads as
   * an international number or an email address.
   */
  public boolean isContact(String name, String value) {
    if (name != null) {
      String lower = name.toLowerCase(Locale.ROOT);
      if (MessagingAttributes.CONTACT_ATTRIBUTES.contains(name)
          || lower.equals("email")
          || lower.endsWith(".email")
          || lower.contains("mobile")
          || lower.contains("phone")
          || lower.contains("whatsapp")
          || lower.contains("viber")
          || lower.contains("messenger")) {
        return true;
      }
    }
    return value != null && (NUMBER.matcher(value).find() || EMAIL.matcher(value).find());
  }
}
