// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

/** Physical format of a document, which the capture page shapes its guide after. */
public enum DocumentFormat {
  /** ISO/IEC 7810 ID-1 card, 85.60 × 53.98 mm: national IDs, driver's licenses. */
  ID_1,
  /** ICAO 9303 TD3 passport data page, 125 × 88 mm. */
  TD3;

  /** The format of the documents the given OCR type reads. */
  static DocumentFormat ofOcrType(String ocrType) {
    return OcrSettings.DEFAULT_OCR_TYPE.equals(ocrType) ? TD3 : ID_1;
  }
}
