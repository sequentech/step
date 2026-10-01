// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

/** Message keys shown to the voter when the identity verification does not succeed. */
public enum ScanovateError {
  INTERNAL("scanovateInternalError"),
  VERIFICATION_FAILED("scanovateVerificationFailedError"),
  DOCUMENT_AUTHENTICATION("scanovateDocumentAuthenticationError"),
  DOCUMENT_UNREADABLE("scanovateDocumentUnreadableError"),
  ATTRIBUTES("scanovateAttributesError"),
  SCORING("scanovateScoringError"),
  MAX_RETRIES("scanovateMaxRetriesError"),
  CAPTURE_INVALID("scanovateCaptureInvalidError"),
  LIVENESS_FAILED("scanovateLivenessError"),
  FACE_MISMATCH("scanovateFaceMismatchError"),
  FACE_NOT_FOUND("scanovateFaceNotFoundError");

  private final String messageKey;

  ScanovateError(String messageKey) {
    this.messageKey = messageKey;
  }

  public String messageKey() {
    return messageKey;
  }
}
