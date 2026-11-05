// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.net.URI;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;

/**
 * How the document photo is checked with the on-premise Scanovate Document Liveness service.
 *
 * @param url internal base URL of the Document Liveness service, only reached by Keycloak
 * @param pipelines the checks to run, which the service must have enabled
 * @param calibration of every check: REGULAR, SOFT or HARD
 * @param minProbability minimum probability, from 0 to 1, that each check gives a physical document
 */
public record DocumentLivenessSettings(
    URI url, List<String> pipelines, String calibration, double minProbability) {
  /** Screen replay, printed copy, portrait substitution and digital manipulation. */
  static final String DEFAULT_PIPELINES = "default-sr,default-pc,default-ps,default-dm";

  static final String DEFAULT_CALIBRATION = "REGULAR";
  static final List<String> CALIBRATIONS = List.of("REGULAR", "SOFT", "HARD");
  static final double DEFAULT_MIN_PROBABILITY = 0.5;

  private static final Pattern PIPELINE = Pattern.compile("[a-z0-9][a-z0-9_.-]{0,63}");

  /**
   * Reads the settings from the authenticator configuration.
   *
   * @throws ScanovateException if the URL is missing, or a setting is malformed
   */
  public static DocumentLivenessSettings fromConfig(Map<String, String> config)
      throws ScanovateException {
    return new DocumentLivenessSettings(
        ServiceUrls.parse(config, ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_URL),
        pipelines(config.get(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_PIPELINES)),
        calibration(config.get(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_CALIBRATION)),
        minProbability(
            config.get(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_MIN_PROBABILITY)));
  }

  private static List<String> pipelines(String configuration) throws ScanovateException {
    String raw =
        configuration == null || configuration.isBlank() ? DEFAULT_PIPELINES : configuration;
    List<String> pipelines =
        Arrays.stream(raw.split(",")).map(String::trim).filter(p -> !p.isEmpty()).toList();
    if (pipelines.isEmpty()) {
      throw new ScanovateException("Document liveness needs at least one pipeline");
    }
    for (String pipeline : pipelines) {
      if (!PIPELINE.matcher(pipeline).matches()) {
        throw new ScanovateException("Invalid document liveness pipeline: " + pipeline);
      }
    }
    return pipelines.stream().distinct().toList();
  }

  private static String calibration(String configuration) throws ScanovateException {
    if (configuration == null || configuration.isBlank()) {
      return DEFAULT_CALIBRATION;
    }
    if (!Set.copyOf(CALIBRATIONS).contains(configuration.trim())) {
      throw new ScanovateException("Invalid document liveness calibration: " + configuration);
    }
    return configuration.trim();
  }

  private static double minProbability(String configuration) throws ScanovateException {
    if (configuration == null || configuration.isBlank()) {
      return DEFAULT_MIN_PROBABILITY;
    }
    double value;
    try {
      value = Double.parseDouble(configuration.trim());
    } catch (NumberFormatException e) {
      throw new ScanovateException("Invalid document liveness minimum probability", e);
    }
    if (!(value > 0 && value <= 1)) {
      throw new ScanovateException("Document liveness minimum probability must be in (0, 1]");
    }
    return value;
  }
}
