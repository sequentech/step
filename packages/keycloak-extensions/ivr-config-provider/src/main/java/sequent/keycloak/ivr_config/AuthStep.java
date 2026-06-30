// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.ivr_config;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * One ordered auth step as returned by {@code GET /realms/{realm}/ivr-config}. Mirrors the same
 * struct deserialized by the IVR Lambda.
 *
 * <p>{@code promptKey} is optional and omitted from the JSON when null.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record AuthStep(
    @JsonProperty(Constants.AUTH_STEP_PROP_FIELD) String field,
    @JsonProperty(Constants.AUTH_STEP_PROP_MAX_DIGITS) int maxDigits,
    @JsonProperty(Constants.AUTH_STEP_PROP_KIND) String kind,
    @JsonProperty(Constants.AUTH_STEP_PROP_MAPS_TO) String mapsTo,
    @JsonProperty(Constants.AUTH_STEP_PROP_PROMPT_KEY) String promptKey) {}
