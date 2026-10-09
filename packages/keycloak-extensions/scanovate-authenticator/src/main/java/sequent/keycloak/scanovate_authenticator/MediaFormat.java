// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/** Accepted file formats, recognised by their signature rather than by what the browser claims. */
public enum MediaFormat {
  JPEG(new byte[] {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF});

  private final byte[] signature;

  MediaFormat(byte[] signature) {
    this.signature = signature;
  }

  private boolean matches(byte[] content) {
    return content.length >= signature.length
        && Arrays.equals(content, 0, signature.length, signature, 0, signature.length);
  }

  /** Returns the format whose signature the content starts with. */
  public static Optional<MediaFormat> detect(byte[] content) {
    return Arrays.stream(values()).filter(format -> format.matches(content)).findFirst();
  }
}
