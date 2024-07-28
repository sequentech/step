// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2020 Cooper Lee
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.credential;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import jakarta.validation.constraints.NotNull;
import java.io.IOException;
import java.util.Optional;
import lombok.Getter;
import org.keycloak.common.util.Time;
import org.keycloak.credential.CredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.dto.OTPSecretData;
import org.keycloak.util.JsonSerialization;

/**
 * Credential model for MessageOTP
 *
 * <p>This credential model simply stores the fact that it is setup in the credential data and
 * doesn't store any secret data, because it's not needed.
 */
public class MessageOTPCredentialModel extends CredentialModel {

  public static final String TYPE = "message-otp";
  private final MessageOTPCredentialData credentialData;
  private final MessageOTPSecretData secretData;

  public MessageOTPCredentialModel(
      MessageOTPCredentialData credentialData, MessageOTPSecretData secretData) {
    this.credentialData = credentialData;
    this.secretData = secretData;
  }

  private static Optional<CredentialModel> getOtpCredentialModel(@NotNull UserModel user) {
    return user.credentialManager()
        .getStoredCredentialsByTypeStream(MessageOTPCredentialModel.TYPE)
        .findFirst();
  }

  public static Optional<MessageOTPCredentialModel.MessageOTPCredentialData>
      getMessageOTPCredentialData(@NotNull UserModel user) {
    return getOtpCredentialModel(user)
        .map(
            credentialModel -> {
              try {
                return JsonSerialization.readValue(
                    credentialModel.getCredentialData(),
                    MessageOTPCredentialModel.MessageOTPCredentialData.class);
              } catch (IOException error) {
                throw new IllegalArgumentException(error);
              }
            });
  }

  public static void updateOtpCredential(
      @NotNull UserModel user,
      @NotNull MessageOTPCredentialModel.MessageOTPCredentialData credentialData,
      String secretValue) {
    getOtpCredentialModel(user)
        .ifPresent(
            credential -> {
              try {
                credential.setCredentialData(JsonSerialization.writeValueAsString(credentialData));
                credential.setSecretData(
                    JsonSerialization.writeValueAsString(new OTPSecretData(secretValue)));
                MessageOTPCredentialModel credentialModel =
                    MessageOTPCredentialModel.createFromCredentialModel(credential);
                user.credentialManager().updateStoredCredential(credentialModel);
              } catch (IOException ioError) {
                throw new RuntimeException(ioError);
              }
            });
  }

  public static MessageOTPCredentialModel create(boolean isSetup) {
    MessageOTPCredentialData credentialData = new MessageOTPCredentialData(isSetup);
    MessageOTPSecretData secretData = new MessageOTPSecretData("nothing");
    MessageOTPCredentialModel credentialModel =
        new MessageOTPCredentialModel(credentialData, secretData);
    credentialModel.fillCredentialModelFields();
    return credentialModel;
  }

  public static MessageOTPCredentialModel createFromCredentialModel(
      CredentialModel credentialModel) {
    try {
      MessageOTPCredentialData credentialData =
          JsonSerialization.readValue(
              credentialModel.getCredentialData(), MessageOTPCredentialData.class);
      MessageOTPSecretData secretData =
          JsonSerialization.readValue(credentialModel.getSecretData(), MessageOTPSecretData.class);
      MessageOTPCredentialModel credential =
          new MessageOTPCredentialModel(credentialData, secretData);

      credential.setUserLabel(credentialModel.getUserLabel());
      credential.setCreatedDate(credentialModel.getCreatedDate());
      credential.setType(TYPE);
      credential.setId(credentialModel.getId());
      credential.setSecretData(credentialModel.getSecretData());
      credential.setCredentialData(credentialModel.getCredentialData());

      return credential;
    } catch (IOException error) {
      throw new RuntimeException(error);
    }
  }

  private void fillCredentialModelFields() {
    try {
      setCredentialData(JsonSerialization.writeValueAsString(credentialData));
      setSecretData(JsonSerialization.writeValueAsString(secretData));
      setType(TYPE);
      setCreatedDate(Time.currentTimeMillis());
    } catch (IOException e) {
      throw new RuntimeException(e);
    }
  }

  public MessageOTPCredentialData getOTPCredentialData() {
    return credentialData;
  }

  public MessageOTPSecretData getOTPSecretData() {
    return secretData;
  }

  @Getter
  public static class MessageOTPCredentialData {
    private final boolean isSetup;

    @JsonIgnore
    public boolean isSecretInvalid() {
      return false;
    }

    @JsonCreator
    public MessageOTPCredentialData(@JsonProperty("isSetup") boolean isSetup) {
      this.isSetup = isSetup;
    }
  }

  public static class EmptySecretData {}
}
