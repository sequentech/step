// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.google.zxing.BarcodeFormat;
import com.google.zxing.EncodeHintType;
import com.google.zxing.WriterException;
import com.google.zxing.common.BitMatrix;
import com.google.zxing.qrcode.QRCodeWriter;
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.Map;
import java.util.Optional;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;

/**
 * The QR code of a Messenger link, for voters on a computer to open the chat on their phone. It is
 * drawn on the server (zxing, shipped with Keycloak) as an SVG data URI, so the page needs no
 * script and both login themes show the same image.
 */
@JBossLog
@UtilityClass
public class MessengerQr {
  public final String DATA_URI_PREFIX = "data:image/svg+xml;base64,";

  /** The QR code of an http(s) link as an SVG data URI; empty for anything else. */
  public Optional<String> dataUri(String link) {
    if (link == null || !(link.startsWith("https://") || link.startsWith("http://"))) {
      return Optional.empty();
    }
    BitMatrix matrix;
    try {
      matrix =
          new QRCodeWriter()
              .encode(
                  link,
                  BarcodeFormat.QR_CODE,
                  0,
                  0,
                  Map.of(
                      EncodeHintType.MARGIN,
                      2,
                      EncodeHintType.ERROR_CORRECTION,
                      ErrorCorrectionLevel.M,
                      EncodeHintType.CHARACTER_SET,
                      "UTF-8"));
    } catch (WriterException | IllegalArgumentException e) {
      log.warn("dataUri(): the Messenger link could not be drawn as a QR code");
      return Optional.empty();
    }
    return Optional.of(
        DATA_URI_PREFIX
            + Base64.getEncoder().encodeToString(svg(matrix).getBytes(StandardCharsets.UTF_8)));
  }

  /** One square per dark module, drawn as a single path. */
  String svg(BitMatrix matrix) {
    int width = matrix.getWidth();
    int height = matrix.getHeight();
    StringBuilder path = new StringBuilder();
    for (int y = 0; y < height; y++) {
      for (int x = 0; x < width; x++) {
        if (matrix.get(x, y)) {
          path.append('M').append(x).append(' ').append(y).append("h1v1h-1z");
        }
      }
    }
    return "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 "
        + width
        + " "
        + height
        + "\" shape-rendering=\"crispEdges\"><rect width=\"100%\" height=\"100%\" fill=\"#fff\"/>"
        + "<path fill=\"#000\" d=\""
        + path
        + "\"/></svg>";
  }
}
