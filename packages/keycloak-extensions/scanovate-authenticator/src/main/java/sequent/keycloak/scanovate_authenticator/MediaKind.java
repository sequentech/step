// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

/** Files captured in the voter's browser in the embedded mode. */
public enum MediaKind {
  FRONT_IMAGE("front", "front_image", MediaCategory.IMAGE),
  BACK_IMAGE("back", "back_image", MediaCategory.IMAGE),
  FACE_IMAGE("face", "face_image", MediaCategory.IMAGE),
  SCAN_VIDEO("video", "scan_video", MediaCategory.VIDEO);

  private final String formPart;
  private final String uploadPart;
  private final MediaCategory category;

  MediaKind(String formPart, String uploadPart, MediaCategory category) {
    this.formPart = formPart;
    this.uploadPart = uploadPart;
    this.category = category;
  }

  /** Name of the multipart part posted by the capture page. */
  public String formPart() {
    return formPart;
  }

  /** Name of the multipart part sent to B-Trust. */
  public String uploadPart() {
    return uploadPart;
  }

  public MediaCategory category() {
    return category;
  }
}
