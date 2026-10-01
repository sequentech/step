// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Map;

/** Body of {@code POST /flow/v3/link}. */
public record LinkRequest(
    int flowId,
    String identifierId,
    String idNumber,
    String redirectUrl,
    Map<String, String> params,
    SaveOption saveOption) {}
