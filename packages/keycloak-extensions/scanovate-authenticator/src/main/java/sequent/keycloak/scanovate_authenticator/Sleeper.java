// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

/** Waits between retries; injectable so that tests don't sleep. */
@FunctionalInterface
public interface Sleeper {
  void sleep(long millis) throws InterruptedException;
}
