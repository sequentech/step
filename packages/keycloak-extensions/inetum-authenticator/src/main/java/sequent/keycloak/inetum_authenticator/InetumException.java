// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.inetum_authenticator;

public class InetumException extends Exception {
  private String error;

  public String getError() {
    return error;
  }

  public InetumException(String ftlErrorAuthInvalid) {
    this.error = ftlErrorAuthInvalid;
  }
}
