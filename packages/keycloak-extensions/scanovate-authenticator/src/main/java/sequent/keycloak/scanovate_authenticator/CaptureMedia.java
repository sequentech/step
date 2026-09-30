// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.io.IOException;
import java.io.InputStream;
import java.util.Collections;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;
import org.keycloak.http.FormPartValue;

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
   * Reads and checks the files posted by the capture page.
   *
   * <p>Parts not required by the settings, such as the back of a single sided document, are
   * ignored.
   *
   * @param parts multipart parts of the request, by name
   * @throws InvalidCaptureException if a required file is missing, repeated, unreadable, too big or
   *     not of an accepted format
   */
  public static CaptureMedia fromParts(
      Map<String, List<FormPartValue>> parts, CaptureSettings settings)
      throws InvalidCaptureException {
    Map<MediaKind, MediaFile> files = new EnumMap<>(MediaKind.class);
    for (MediaKind kind : settings.requiredMedia()) {
      List<FormPartValue> values = parts.get(kind.formPart());
      if (values == null || values.isEmpty()) {
        throw new InvalidCaptureException("missing " + kind.formPart());
      }
      if (values.size() > 1) {
        throw new InvalidCaptureException("repeated " + kind.formPart());
      }
      int maxBytes = settings.maxBytes(kind.category());
      byte[] content = read(kind, values.get(0), maxBytes);
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

  /**
   * Reads up to one byte past the limit, so that bigger files are detected without reading them.
   */
  private static byte[] read(MediaKind kind, FormPartValue value, int maxBytes)
      throws InvalidCaptureException {
    try (InputStream input = value.asInputStream()) {
      if (input == null) {
        throw new InvalidCaptureException("missing " + kind.formPart());
      }
      return input.readNBytes(maxBytes + 1);
    } catch (IOException e) {
      throw new InvalidCaptureException("could not read " + kind.formPart(), e);
    }
  }
}
