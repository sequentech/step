// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package com.example;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

class ECIESEncryptionToolTest {

    @Test
    void literalArgumentIsReturnedUnchanged() {
        assertEquals(
            "pass word; $(id)",
            ECIESEncryptionTool.resolveSecretArgument("pass word; $(id)", Collections.emptyMap()));
    }

    @Test
    void envArgumentIsReadFromTheEnvironment() {
        Map<String, String> environment = new HashMap<>();
        environment.put("ECIES_SECRET", "s3cret value");
        assertEquals(
            "s3cret value",
            ECIESEncryptionTool.resolveSecretArgument("env:ECIES_SECRET", environment));
    }

    @Test
    void envArgumentMayResolveToAnEmptyValue() {
        Map<String, String> environment = new HashMap<>();
        environment.put("ECIES_SECRET", "");
        assertEquals("", ECIESEncryptionTool.resolveSecretArgument("env:ECIES_SECRET", environment));
    }

    @Test
    void unsetVariableFailsNamingTheVariableButNoValue() {
        Map<String, String> environment = new HashMap<>();
        environment.put("OTHER", "other-secret");
        IllegalArgumentException error = assertThrows(
            IllegalArgumentException.class,
            () -> ECIESEncryptionTool.resolveSecretArgument("env:ECIES_SECRET", environment));
        assertTrue(error.getMessage().contains("ECIES_SECRET"));
        assertFalse(error.getMessage().contains("other-secret"));
    }

    @Test
    void missingVariableNameFails() {
        assertThrows(
            IllegalArgumentException.class,
            () -> ECIESEncryptionTool.resolveSecretArgument("env:", Collections.emptyMap()));
    }
}
