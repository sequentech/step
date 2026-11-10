// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import com.fasterxml.jackson.annotation.JsonProperty;
import lombok.Data;

@Data
public class SmartLinkRequest {
  @JsonProperty("username")
  private String username;

  @JsonProperty("email_or_username")
  private String emailOrUsername;

  @JsonProperty("client_id")
  private String clientId;

  @JsonProperty("redirect_uri")
  private String redirectUri;

  @JsonProperty("expiration_seconds")
  private int expirationSeconds = 60 * 60 * 24;

  @JsonProperty("force_create")
  private boolean forceCreate = false;

  @JsonProperty("update_profile")
  private boolean updateProfile = false;

  @JsonProperty("update_password")
  private boolean updatePassword = false;

  @JsonProperty("send_notification")
  private boolean sendNotification = false;

  @JsonProperty("scopes")
  private String scopes = null;

  @JsonProperty("nonce")
  private String nonce = null;

  @JsonProperty("state")
  private String state = null;

  @JsonProperty("remember_me")
  private Boolean rememberMe = false;

  @JsonProperty("reusable")
  private Boolean actionTokenPersistent = true;

  @JsonProperty("mark_email_verified")
  private Boolean markEmailVerified = true;
}
