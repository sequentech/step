// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.ivr_config;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * One ordered auth step as returned by {@code GET /realms/{realm}/ivr-config}.
 * Mirrors the same struct deserialized by the IVR Lambda.
 * <p>
 * {@code promptKey} is optional and omitted from the JSON when null.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record AuthStep(
    @JsonProperty("field") String field,
    @JsonProperty("max_digits") int maxDigits,
    @JsonProperty("terminator") String terminator,
    @JsonProperty("maps_to") String mapsTo,
    @JsonProperty("prompt_key") String promptKey) {}
