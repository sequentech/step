// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Optional;

/** Accepted file formats, recognised by their signature rather than by what the browser claims. */
public enum MediaFormat {
  JPEG(
      MediaCategory.IMAGE,
      "image/jpeg",
      "jpg",
      0,
      new byte[] {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF}),
  WEBM(
      MediaCategory.VIDEO,
      "video/webm",
      "webm",
      0,
      new byte[] {0x1A, 0x45, (byte) 0xDF, (byte) 0xA3}),
  MP4(MediaCategory.VIDEO, "video/mp4", "mp4", 4, "ftyp".getBytes(StandardCharsets.US_ASCII));

  private final MediaCategory category;
  private final String contentType;
  private final String extension;
  private final int signatureOffset;
  private final byte[] signature;

  MediaFormat(
      MediaCategory category,
      String contentType,
      String extension,
      int signatureOffset,
      byte[] signature) {
    this.category = category;
    this.contentType = contentType;
    this.extension = extension;
    this.signatureOffset = signatureOffset;
    this.signature = signature;
  }

  public MediaCategory category() {
    return category;
  }

  public String contentType() {
    return contentType;
  }

  public String extension() {
    return extension;
  }

  private boolean matches(byte[] content) {
    int end = signatureOffset + signature.length;
    return content.length >= end
        && Arrays.equals(content, signatureOffset, end, signature, 0, signature.length);
  }

  /** Returns the format of the given category whose signature the content starts with. */
  public static Optional<MediaFormat> detect(MediaCategory category, byte[] content) {
    return Arrays.stream(values())
        .filter(format -> format.category == category && format.matches(content))
        .findFirst();
  }
}
