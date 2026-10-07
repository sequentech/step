// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/** Side of the identity document that the voter captures. */
public enum DocumentSide {
  FRONT("front", MediaKind.FRONT_IMAGE),
  BACK("back", MediaKind.BACK_IMAGE);

  private final String value;
  private final MediaKind mediaKind;

  DocumentSide(String value, MediaKind mediaKind) {
    this.value = value;
    this.mediaKind = mediaKind;
  }

  public String value() {
    return value;
  }

  public MediaKind mediaKind() {
    return mediaKind;
  }

  public static Optional<DocumentSide> fromValue(String value) {
    return Arrays.stream(values()).filter(side -> side.value.equals(value)).findFirst();
  }
}
