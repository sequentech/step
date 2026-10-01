// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/** Files captured in the voter's browser in the embedded mode. */
public enum MediaKind {
  FRONT_IMAGE("front", "front_image", MediaCategory.IMAGE),
  BACK_IMAGE("back", "back_image", MediaCategory.IMAGE),
  FACE_IMAGE("face", "face_image", MediaCategory.IMAGE),
  SCAN_VIDEO("video", "scan_video", MediaCategory.VIDEO),
  /**
   * The voter holding the ID next to their face, with the liveness face capture. Only compared with
   * the voter's live face by Face Match, never sent to B-Trust.
   */
  HOLDING_IMAGE("holding", "holding_image", MediaCategory.IMAGE);

  private final String formPart;
  private final String uploadPart;
  private final MediaCategory category;

  MediaKind(String formPart, String uploadPart, MediaCategory category) {
    this.formPart = formPart;
    this.uploadPart = uploadPart;
    this.category = category;
  }

  /** Name of the part the capture page uploads, see {@link CaptureUploads}. */
  public String formPart() {
    return formPart;
  }

  public static Optional<MediaKind> fromFormPart(String formPart) {
    return Arrays.stream(values()).filter(kind -> kind.formPart.equals(formPart)).findFirst();
  }

  /** Name of the multipart part sent to B-Trust, see {@link ScanovateClient#uploadMedia}. */
  public String uploadPart() {
    return uploadPart;
  }

  public MediaCategory category() {
    return category;
  }
}
