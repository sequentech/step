// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

class AuditEventPersistenceException extends RuntimeException {

  AuditEventPersistenceException(String message) {
    super(message);
  }

  AuditEventPersistenceException(String message, Throwable cause) {
    super(message, cause);
  }
}
