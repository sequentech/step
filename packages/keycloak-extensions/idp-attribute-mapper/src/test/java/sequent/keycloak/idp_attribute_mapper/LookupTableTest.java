// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_attribute_mapper;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Locale;
import java.util.function.UnaryOperator;
import org.junit.jupiter.api.Test;

class LookupTableTest {

  private static LookupTable parse(String text) {
    return LookupTable.parse(text, UnaryOperator.identity());
  }

  @Test
  void groupedRowsMapEveryKey() {
    LookupTable table = parse("area-a = 01:0001, 01:0002\narea-b = 02:0001\n");
    assertEquals("area-a", table.lookup("01:0001"));
    assertEquals("area-a", table.lookup("01:0002"));
    assertEquals("area-b", table.lookup("02:0001"));
    assertNull(table.lookup("02:0002"));
  }

  @Test
  void csvRowsMapOneKeyEach() {
    LookupTable table = parse("01:0001,area-a\n01:0002 , area-b\n");
    assertEquals("area-a", table.lookup("01:0001"));
    assertEquals("area-b", table.lookup("01:0002"));
  }

  @Test
  void commentsAndBlankLinesAreIgnored() {
    LookupTable table = parse("# header\n\n  \narea-a = x\r\n# trailing\n");
    assertEquals("area-a", table.lookup("x"));
    assertEquals(1, table.size());
  }

  @Test
  void paddedRangeRequiresSameWidth() {
    LookupTable table = parse("area-a = 09:0001..09:0420");
    assertEquals("area-a", table.lookup("09:0001"));
    assertEquals("area-a", table.lookup("09:0123"));
    assertEquals("area-a", table.lookup("09:0420"));
    assertNull(table.lookup("09:0421"));
    assertNull(table.lookup("09:0000"));
    assertNull(table.lookup("09:123"));
    assertNull(table.lookup("10:0123"));
  }

  @Test
  void rangeEndCanBeDigitsOnly() {
    LookupTable table = parse("area-a = 09:0001..0420");
    assertEquals("area-a", table.lookup("09:0420"));
  }

  @Test
  void unpaddedRangeAcceptsAnyWidth() {
    LookupTable table = parse("area-a = s1..s250");
    assertEquals("area-a", table.lookup("s7"));
    assertEquals("area-a", table.lookup("s007"));
    assertEquals("area-a", table.lookup("s250"));
    assertNull(table.lookup("s251"));
  }

  @Test
  void severalRangesPerPrefix() {
    LookupTable table =
        parse("area-a = 09:0001..09:0100, 09:0301..09:0400\narea-b = 09:0101..09:0300");
    assertEquals("area-a", table.lookup("09:0050"));
    assertEquals("area-b", table.lookup("09:0101"));
    assertEquals("area-b", table.lookup("09:0300"));
    assertEquals("area-a", table.lookup("09:0301"));
    assertNull(table.lookup("09:0401"));
  }

  @Test
  void exactKeyWinsOverRange() {
    LookupTable table = parse("area-a = 09:0001..09:0100\narea-b = 09:0050");
    assertEquals("area-b", table.lookup("09:0050"));
  }

  @Test
  void keysAreNormalized() {
    LookupTable table =
        LookupTable.parse("area-a = AB:0001..AB:0010", s -> s.toLowerCase(Locale.ROOT));
    assertEquals("area-a", table.lookup("ab:0005"));
    assertNull(table.lookup("AB:0005"));
  }

  @Test
  void keyWithoutDigitsDoesNotMatchRanges() {
    LookupTable table = parse("area-a = a1..a9");
    assertNull(table.lookup("a"));
    assertNull(table.lookup(""));
  }

  @Test
  void conflictingKeyIsRejected() {
    var e = assertThrows(LookupTable.ParseException.class, () -> parse("a = k\nb = k"));
    assertTrue(e.getMessage().contains("line 2"));
  }

  @Test
  void repeatedKeyWithSameValueIsAccepted() {
    assertEquals("a", parse("a = k\na = k").lookup("k"));
  }

  @Test
  void overlappingRangesAreRejected() {
    assertThrows(LookupTable.ParseException.class, () -> parse("a = 01..10\nb = 10..20"));
  }

  @Test
  void invalidLinesAreRejected() {
    assertThrows(LookupTable.ParseException.class, () -> parse("no separator here"));
    assertThrows(LookupTable.ParseException.class, () -> parse("a,b,c"));
    assertThrows(LookupTable.ParseException.class, () -> parse(" = k"));
    assertThrows(LookupTable.ParseException.class, () -> parse("a = k,,j"));
    assertThrows(LookupTable.ParseException.class, () -> parse("a = x..10"));
    assertThrows(LookupTable.ParseException.class, () -> parse("a = x1..y10"));
    assertThrows(LookupTable.ParseException.class, () -> parse("a = 10..1"));
    assertThrows(LookupTable.ParseException.class, () -> parse("a = 1..1234567890123456789"));
  }

  @Test
  void largeTableParsesAndLooksUpQuickly() {
    StringBuilder text = new StringBuilder();
    for (int region = 1; region <= 32; region++) {
      for (int block = 0; block < 100; block++) {
        int from = block * 25 + 1;
        text.append(
            String.format(
                "area-%02d-%03d = %02d:%04d..%04d%n", region, block, region, from, from + 24));
      }
    }
    LookupTable table = parse(text.toString());
    assertEquals(3200, table.size());
    assertEquals("area-17-040", table.lookup("17:1001"));
    assertNull(table.lookup("33:0001"));
  }
}
