// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.net.URI;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class FaceMatchSettingsTest {
  private static Map<String, String> config() {
    Map<String, String> config = new HashMap<>();
    config.put(ScanovateAuthenticatorFactory.FACE_MATCH_URL, "http://face-match:3000/");
    return config;
  }

  @Test
  void defaultsApplyWhenUnset() throws ScanovateException {
    FaceMatchSettings settings = FaceMatchSettings.fromConfig(config(), "passport");

    assertEquals(URI.create("http://face-match:3000"), settings.url());
    assertEquals(0.67, settings.minSimilarity());
  }

  @Test
  void minimumSimilarityIsReadForTheDocumentType() throws ScanovateException {
    Map<String, String> config = config();
    config.put(
        ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY,
        "{\"passport\": 0.8, \"default\": 0.7}");

    assertEquals(0.8, FaceMatchSettings.fromConfig(config, "passport").minSimilarity());
    assertEquals(0.7, FaceMatchSettings.fromConfig(config, "driversLicense").minSimilarity());
    assertEquals(0.7, FaceMatchSettings.fromConfig(config, null).minSimilarity());
  }

  @Test
  void documentTypesWithoutAnEntryUseTheDefaultMinimum() throws ScanovateException {
    Map<String, String> config = config();
    config.put(ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "{\"passport\": 0.8}");

    assertEquals(0.67, FaceMatchSettings.fromConfig(config, "iBP").minSimilarity());
  }

  @Test
  void missingOrInvalidSettingsAreRejected() {
    for (Map.Entry<String, String> invalid :
        List.of(
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_URL, ""),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_URL, "face-match:3000"),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_URL, "ftp://face-match"),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "0.7"),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "{"),
            Map.entry(ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "{\"default\": 0}"),
            Map.entry(
                ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "{\"default\": 1.5}"),
            Map.entry(
                ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY, "{\"default\": \"0.7\"}"),
            Map.entry(
                ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY,
                "{\"default\": 0.7, \"passport\": -1}"))) {
      Map<String, String> config = config();
      config.put(invalid.getKey(), invalid.getValue());
      assertThrows(
          ScanovateException.class,
          () -> FaceMatchSettings.fromConfig(config, "passport"),
          invalid.toString());
    }
    assertThrows(ScanovateException.class, () -> FaceMatchSettings.fromConfig(Map.of(), "x"));
  }
}
