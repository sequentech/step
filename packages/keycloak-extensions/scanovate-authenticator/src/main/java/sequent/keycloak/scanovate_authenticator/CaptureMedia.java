// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Collections;
import java.util.EnumMap;
import java.util.Map;

/**
 * Photos captured in the voter's browser, checked and ready to be compared and read on premise.
 *
 * <p>The browser only guides the voter: nothing it reports about the quality of the capture is
 * trusted. The server only checks that the expected photos are there, that they are JPEG images and
 * that they fit the size limit. The voter's face is checked by Liveness Plus and Face Match, and
 * the document is read by the OCR service.
 *
 * @param files captured photos, in capture order
 */
public record CaptureMedia(Map<MediaKind, byte[]> files) {
  public CaptureMedia {
    files = Collections.unmodifiableMap(new EnumMap<>(files));
  }

  /**
   * Checks the photos uploaded by the capture page, see {@link CaptureUploads}.
   *
   * <p>Parts not required by the settings, such as the back of a single sided document, are
   * ignored.
   *
   * @param parts uploaded files, by part name
   * @throws InvalidCaptureException if a required photo is missing, too big or not a JPEG image
   */
  public static CaptureMedia fromUploads(Map<String, byte[]> parts, CaptureSettings settings)
      throws InvalidCaptureException {
    Map<MediaKind, byte[]> files = new EnumMap<>(MediaKind.class);
    for (MediaKind kind : settings.requiredMedia()) {
      byte[] content = parts.get(kind.formPart());
      if (content == null || content.length == 0) {
        throw new InvalidCaptureException("missing " + kind.formPart());
      }
      if (content.length > settings.maxImageBytes()) {
        throw new InvalidCaptureException(
            kind.formPart() + " is larger than " + settings.maxImageBytes());
      }
      if (MediaFormat.detect(content).isEmpty()) {
        throw new InvalidCaptureException(kind.formPart() + " has an invalid format");
      }
      files.put(kind, content);
    }
    return new CaptureMedia(files);
  }
}
