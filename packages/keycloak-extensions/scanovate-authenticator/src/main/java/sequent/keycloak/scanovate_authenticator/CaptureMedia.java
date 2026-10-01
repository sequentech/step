// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Collections;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;

/**
 * Files captured in the voter's browser, checked and ready to be compared or sent to B-Trust.
 *
 * <p>The browser only guides the voter: nothing it reports about the quality of the capture is
 * trusted. The server only checks that the expected files are there, that they are what they claim
 * to be and that they fit the size limits. B-Trust verifies the document and, with the photo face
 * capture, the voter's face. With the liveness face capture, the voter's face is checked on premise
 * instead, by Liveness Plus and Face Match.
 *
 * @param files captured files, in upload order
 */
public record CaptureMedia(Map<MediaKind, MediaFile> files) {
  /** A captured file and its format, detected from its content. */
  public record MediaFile(MediaFormat format, byte[] content) {}

  public CaptureMedia {
    files = Collections.unmodifiableMap(new EnumMap<>(files));
  }

  /**
   * Checks the files uploaded by the capture page, see {@link CaptureUploads}.
   *
   * <p>Parts not required by the settings, such as the back of a single sided document, are
   * ignored.
   *
   * @param parts uploaded files, by part name
   * @throws InvalidCaptureException if a required file is missing, too big or not of an accepted
   *     format
   */
  public static CaptureMedia fromUploads(Map<String, byte[]> parts, CaptureSettings settings)
      throws InvalidCaptureException {
    Map<MediaKind, MediaFile> files = new EnumMap<>(MediaKind.class);
    for (MediaKind kind : settings.requiredMedia()) {
      byte[] content = parts.get(kind.formPart());
      if (content == null || content.length == 0) {
        throw new InvalidCaptureException("missing " + kind.formPart());
      }
      int maxBytes = settings.maxBytes(kind.category());
      if (content.length > maxBytes) {
        throw new InvalidCaptureException(kind.formPart() + " is larger than " + maxBytes);
      }
      MediaFormat format =
          MediaFormat.detect(kind.category(), content)
              .orElseThrow(
                  () -> new InvalidCaptureException(kind.formPart() + " has an invalid format"));
      files.put(kind, new MediaFile(format, content));
    }
    return new CaptureMedia(files);
  }

  /** Returns a copy with only the given files, such as those sent to B-Trust. */
  public CaptureMedia only(List<MediaKind> kinds) {
    Map<MediaKind, MediaFile> kept = new EnumMap<>(MediaKind.class);
    kinds.stream().filter(files::containsKey).forEach(kind -> kept.put(kind, files.get(kind)));
    return new CaptureMedia(kept);
  }
}
