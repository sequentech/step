// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;
import java.util.Iterator;
import java.util.Map;
import java.util.regex.Pattern;

/** The representation of an admin event as it is logged: without secrets and of bounded size. */
final class AdminRepresentation {
  static final String REDACTED = "**redacted**";
  static final int MAX_BYTES = 64 * 1024;

  private static final Pattern SECRET_KEY =
      Pattern.compile(
          "password|secret|private_?key|token|bindCredential|(^|[-_.])(code|otp)$",
          Pattern.CASE_INSENSITIVE);

  /** Matched against the path of keys, so that it finds {@code credentials[].value}. */
  private static final Pattern SECRET_PATH =
      Pattern.compile("credential.*value", Pattern.CASE_INSENSITIVE);

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private AdminRepresentation() {}

  /** Event-realm audit keeps the change, but no network attribution or unstructured fallback. */
  static Object forEventRealmLog(String representation) {
    if (representation == null) return null;
    try {
      JsonNode json = MAPPER.readTree(representation);
      if (json == null || (!json.isObject() && !json.isArray())) return REDACTED;
      redactNetwork(json);
      return forLog(MAPPER.writeValueAsString(json));
    } catch (JsonProcessingException invalidJson) {
      return REDACTED;
    }
  }

  private static void redactNetwork(JsonNode node) {
    if (node instanceof ObjectNode object) {
      object
          .fields()
          .forEachRemaining(
              field -> {
                if (UserEventDetails.maySayNetwork(field.getKey())) {
                  field.setValue(MAPPER.getNodeFactory().textNode(REDACTED));
                } else if (field.getValue().isTextual()) {
                  field.setValue(
                      MAPPER
                          .getNodeFactory()
                          .textNode(
                              UserEventDetails.safeDetailValue(field.getValue().textValue())));
                } else {
                  redactNetwork(field.getValue());
                }
              });
    } else if (node instanceof ArrayNode array) {
      for (int index = 0; index < array.size(); index++) {
        JsonNode item = array.get(index);
        if (item.isTextual()) {
          array.set(index, UserEventDetails.safeDetailValue(item.textValue()));
        } else {
          redactNetwork(item);
        }
      }
    }
  }

  /**
   * The JSON of {@code representation} with its secrets redacted, or its text when it is not JSON.
   * Over {@link #MAX_BYTES} it is replaced by the SHA-256 of what would have been logged.
   */
  static Object forLog(String representation) {
    if (representation == null) {
      return null;
    }
    Object logged;
    String text;
    try {
      JsonNode json = MAPPER.readTree(representation);
      redact(json, "");
      logged = json;
      text = MAPPER.writeValueAsString(json);
    } catch (JsonProcessingException notJson) {
      logged = representation;
      text = representation;
    }
    byte[] bytes = text.getBytes(StandardCharsets.UTF_8);
    if (bytes.length <= MAX_BYTES) {
      return logged;
    }
    ObjectNode summary = MAPPER.createObjectNode();
    summary.put("sha256", sha256(bytes));
    summary.put("truncated", true);
    return summary;
  }

  private static void redact(JsonNode node, String path) {
    if (node instanceof ArrayNode array) {
      array.forEach(item -> redact(item, path));
    } else if (node instanceof ObjectNode object) {
      Iterator<Map.Entry<String, JsonNode>> fields = object.fields();
      while (fields.hasNext()) {
        Map.Entry<String, JsonNode> field = fields.next();
        String fieldPath = path + "/" + field.getKey();
        if (namesASecret(field.getKey(), fieldPath) && mayHoldASecret(field.getValue())) {
          field.setValue(MAPPER.getNodeFactory().textNode(REDACTED));
        } else {
          redact(field.getValue(), fieldPath);
        }
      }
    }
  }

  private static boolean namesASecret(String key, String path) {
    return SECRET_KEY.matcher(key).find() || SECRET_PATH.matcher(path).find();
  }

  /** Lifespans and switches such as accessTokenLifespan are settings, not secrets. */
  private static boolean mayHoldASecret(JsonNode value) {
    return value.isTextual() || value.isContainerNode();
  }

  private static String sha256(byte[] bytes) {
    try {
      return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    } catch (NoSuchAlgorithmException impossible) {
      throw new IllegalStateException(impossible);
    }
  }
}
