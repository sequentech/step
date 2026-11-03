// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import java.util.Map;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;

@UtilityClass
@JBossLog
public class Utils {
  public static final String EVENT_TYPE_COMMUNICATIONS = "communications";

  /** The detail that says what kind of event the authenticator recorded. */
  public static final String EVENT_DETAIL_TYPE = "type";

  /** The detail that holds the text of a message sent to the account. */
  public static final String EVENT_DETAIL_MSG_BODY = "msgBody";

  /** Whether an event's details record a message Keycloak sent to the account. */
  public static boolean isCommunication(Map<String, String> details) {
    return details != null && EVENT_TYPE_COMMUNICATIONS.equals(details.get(EVENT_DETAIL_TYPE));
  }
}
