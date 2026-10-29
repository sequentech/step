// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_attribute_mapper;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;
import sequent.keycloak.idp_attribute_mapper.MapperSettings.MissPolicy;
import sequent.keycloak.idp_attribute_mapper.MapperSettings.MissReason;
import sequent.keycloak.idp_attribute_mapper.MapperSettings.Resolution;
import sequent.keycloak.idp_attribute_mapper.MapperSettings.Target;

class MapperSettingsTest {

  private static final String UUID_V8 =
      "^[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";

  private static Map<String, String> config(String... pairs) {
    Map<String, String> config = new HashMap<>();
    for (int i = 0; i < pairs.length; i += 2) {
      config.put(pairs[i], pairs[i + 1]);
    }
    return config;
  }

  private static Resolution resolve(MapperSettings settings, Map<String, String> sources) {
    return settings.resolve(sources::get);
  }

  @Test
  void extractsValidatesAndLowercases() {
    MapperSettings settings =
        MapperSettings.parse(
            config(
                MapperSettings.SOURCES, MapperSettings.SOURCE_NAME_ID,
                MapperSettings.EXTRACT_REGEX, "^([^@]+)@",
                MapperSettings.CASE, "LOWER",
                MapperSettings.VALIDATION_REGEX, UUID_V8,
                MapperSettings.TARGET, "USERNAME"));
    Resolution hit =
        resolve(
            settings,
            Map.of(
                MapperSettings.SOURCE_NAME_ID, "3F2B8C1A-9D4E-8A7B-B1C2-0D9E8F7A6B5C@example.org"));
    assertTrue(hit.isHit());
    assertEquals("3f2b8c1a-9d4e-8a7b-b1c2-0d9e8f7a6b5c", hit.value());
    assertEquals(Target.USERNAME, settings.target());

    Resolution wrongVersion =
        resolve(
            settings,
            Map.of(
                MapperSettings.SOURCE_NAME_ID, "3f2b8c1a-9d4e-4a7b-b1c2-0d9e8f7a6b5c@example.org"));
    assertEquals(MissReason.VALIDATION_FAILED, wrongVersion.missReason());

    Resolution noDomain =
        resolve(
            settings,
            Map.of(MapperSettings.SOURCE_NAME_ID, "3f2b8c1a-9d4e-8a7b-b1c2-0d9e8f7a6b5c"));
    assertEquals(MissReason.EXTRACT_FAILED, noDomain.missReason());
  }

  @Test
  void regexWithoutGroupKeepsWholeMatch() {
    MapperSettings settings =
        MapperSettings.parse(
            config(
                MapperSettings.SOURCES, "id",
                MapperSettings.EXTRACT_REGEX, "\\d+",
                MapperSettings.TARGET_ATTRIBUTE, "n"));
    assertEquals("123", resolve(settings, Map.of("id", "abc123def")).value());
  }

  @Test
  void joinsSourcesAndLooksUp() {
    MapperSettings settings =
        MapperSettings.parse(
            config(
                MapperSettings.SOURCES, "region##precinct",
                MapperSettings.TARGET_ATTRIBUTE, "area-id",
                MapperSettings.LOOKUP_TABLE, "area-1 = 09:0001..09:0100\narea-2 = 09:0101..09:0200",
                MapperSettings.ON_MISS, "REJECT"));
    assertEquals("area-2", resolve(settings, Map.of("region", " 09 ", "precinct", "0150")).value());
    Resolution miss = resolve(settings, Map.of("region", "09", "precinct", "0201"));
    assertEquals(MissReason.NOT_IN_TABLE, miss.missReason());
    assertEquals("09:0201", miss.input());
    assertEquals(MissPolicy.REJECT, settings.missPolicy());
    assertEquals(MapperSettings.DEFAULT_ERROR_MESSAGE, settings.errorMessage());
  }

  @Test
  void customSeparatorAndUpperCaseKeys() {
    MapperSettings settings =
        MapperSettings.parse(
            config(
                MapperSettings.SOURCES, "a##b",
                MapperSettings.SEPARATOR, "/",
                MapperSettings.CASE, "UPPER",
                MapperSettings.TARGET_ATTRIBUTE, "x",
                MapperSettings.LOOKUP_TABLE, "v = ab/cd"));
    assertEquals("v", resolve(settings, Map.of("a", "Ab", "b", "cD")).value());
  }

  @Test
  void missingSourceIsAMiss() {
    MapperSettings settings =
        MapperSettings.parse(
            config(MapperSettings.SOURCES, "a##b", MapperSettings.TARGET_ATTRIBUTE, "x"));
    Resolution miss = resolve(settings, Map.of("a", "1"));
    assertFalse(miss.isHit());
    assertEquals(MissReason.SOURCE_MISSING, miss.missReason());
    assertEquals(
        MissReason.SOURCE_MISSING, resolve(settings, Map.of("a", "1", "b", "  ")).missReason());
  }

  @Test
  void withoutTableTheJoinedValueIsStored() {
    MapperSettings settings =
        MapperSettings.parse(
            config(MapperSettings.SOURCES, "a", MapperSettings.TARGET_ATTRIBUTE, "x"));
    assertEquals("value", resolve(settings, Map.of("a", " value ")).value());
  }

  @Test
  void invalidConfigurationIsRejected() {
    assertThrows(IllegalArgumentException.class, () -> MapperSettings.parse(config()));
    assertThrows(
        IllegalArgumentException.class,
        () -> MapperSettings.parse(config(MapperSettings.SOURCES, "a")));
    assertThrows(
        IllegalArgumentException.class,
        () ->
            MapperSettings.parse(
                config(
                    MapperSettings.SOURCES, "a",
                    MapperSettings.TARGET_ATTRIBUTE, "x",
                    MapperSettings.EXTRACT_REGEX, "(")));
    assertThrows(
        IllegalArgumentException.class,
        () ->
            MapperSettings.parse(
                config(
                    MapperSettings.SOURCES, "a",
                    MapperSettings.TARGET_ATTRIBUTE, "x",
                    MapperSettings.CASE, "TITLE")));
    assertThrows(
        IllegalArgumentException.class,
        () ->
            MapperSettings.parse(
                config(
                    MapperSettings.SOURCES, "a",
                    MapperSettings.TARGET_ATTRIBUTE, "x",
                    MapperSettings.ON_MISS, "DEFAULT")));
    assertThrows(
        LookupTable.ParseException.class,
        () ->
            MapperSettings.parse(
                config(
                    MapperSettings.SOURCES, "a",
                    MapperSettings.TARGET_ATTRIBUTE, "x",
                    MapperSettings.LOOKUP_TABLE, "garbage")));
  }
}
