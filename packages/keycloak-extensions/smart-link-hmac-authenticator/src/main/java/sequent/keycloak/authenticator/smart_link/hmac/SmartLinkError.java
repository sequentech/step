// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

/**
 * Reasons an HMAC Smart Link can be rejected.
 *
 * <p>These codes are only ever written to the server log. The voter is always shown a single,
 * generic error, so that the endpoint does not leak which specific check failed (which user ids
 * exist, whether the secret was close, etc.).
 */
public enum SmartLinkError {
  /** No shared secret configured on the realm: the feature is effectively disabled. */
  NOT_CONFIGURED,
  /** The {@code khmac:///...} envelope could not be parsed. */
  MALFORMED_TOKEN,
  /** The envelope declared a digest other than {@code sha-256}. */
  UNSUPPORTED_DIGEST,
  /** The signed message did not have the expected {@code user:AuthEvent:eid:vote:ts} shape. */
  MALFORMED_MESSAGE,
  /** The user id field was empty. */
  INVALID_USER_ID,
  /** The permission object/action was not {@code AuthEvent}/{@code vote}. */
  INVALID_PERMISSION,
  /** The election id in the token did not match the realm/path it was presented to. */
  MISMATCHED_EVENT,
  /** The HMAC did not match: wrong secret or tampered message. */
  INVALID_SIGNATURE,
  /** The token timestamp is in the future (beyond the allowed clock skew). */
  TOKEN_IN_FUTURE,
  /** The token timestamp is older than the allowed validity window. */
  TOKEN_EXPIRED,
  /** The user id is not part of the realm census and auto-creation is disabled. */
  USER_NOT_FOUND,
  /** The matched user is disabled. */
  USER_DISABLED,
  /** A configured Smart Link required attribute was missing or did not match the census user. */
  REQUIRED_ATTRIBUTE_MISMATCH,
  /** The configured Smart Link client does not exist in the realm. */
  CLIENT_NOT_FOUND,
  /** The supplied redirect_uri is not allowed by the client. */
  REDIRECT_NOT_ALLOWED,
  /** Any unexpected internal error while establishing the session. */
  INTERNAL_ERROR
}
