// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/** Photos captured in the voter's browser. */
public enum MediaKind {
  /** The front of the document, read by the OCR service and compared with the voter's face. */
  FRONT_IMAGE("front"),
  /** The back of the document, for documents with one, read by the OCR service. */
  BACK_IMAGE("back"),
  /** The voter holding the document next to their face, compared with the voter's face. */
  HOLDING_IMAGE("holding");

  private final String formPart;

  MediaKind(String formPart) {
    this.formPart = formPart;
  }

  /** Name of the part the capture page uploads, see {@link CaptureUploads}. */
  public String formPart() {
    return formPart;
  }

  public static Optional<MediaKind> fromFormPart(String formPart) {
    return Arrays.stream(values()).filter(kind -> kind.formPart.equals(formPart)).findFirst();
  }
}
