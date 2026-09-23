// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package com.example;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import javax.crypto.Cipher;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.*;
import java.security.spec.*;
import java.util.*;
import java.util.concurrent.TimeUnit;
import org.spongycastle.jce.provider.BouncyCastleProvider;
import org.spongycastle.jce.spec.IESParameterSpec;
import static org.junit.jupiter.api.Assertions.*;

class CliContractTest {
    @TempDir Path dir;

    // The real CLI owns System.exit. A bounded child process isolates it and its provider registry.
    private Result cli(String... args) throws Exception {
        List<String> command = new ArrayList<>(Arrays.asList(
            Paths.get(System.getProperty("java.home"), "bin", "java").toString(),
            "-cp", System.getProperty("surefire.test.class.path", System.getProperty("java.class.path")),
            ECIESEncryptionTool.class.getName()));
        command.addAll(Arrays.asList(args));
        Path output = Files.createTempFile(dir, "cli-", ".out");
        Path errors = Files.createTempFile(dir, "cli-", ".err");
        Process child = new ProcessBuilder(command).redirectOutput(output.toFile()).redirectError(errors.toFile()).start();
        try {
            assertTrue(child.waitFor(20, TimeUnit.SECONDS), "CLI timed out");
            return new Result(child.exitValue(), read(output), read(errors));
        } finally {
            if (child.isAlive()) {
                child.destroyForcibly();
                assertTrue(child.waitFor(5, TimeUnit.SECONDS), "CLI did not terminate after forceful cleanup");
            }
        }
    }
    private static String read(Path path) throws Exception {
        return new String(Files.readAllBytes(path), StandardCharsets.UTF_8).trim();
    }
    // A result is the exact stdout; JVM notices such as JAVA_TOOL_OPTIONS go to stderr.
    // Usage is printed on stdout and uncaught exceptions on stderr, so failures search both.
    private static class Result {
        final int status; final String output; final String errors;
        Result(int status, String output, String errors) { this.status = status; this.output = output; this.errors = errors; }
        String success() { assertEquals(0, status, output + "\n" + errors); return output; }
        void failure(String message) {
            String both = output + "\n" + errors;
            assertNotEquals(0, status, both);
            assertTrue(both.contains(message), both);
        }
    }
    private Path write(String name, byte[] bytes) throws Exception { return Files.write(dir.resolve(name), bytes); }
    private Path pem(String name, String kind, byte[] bytes) throws Exception {
        return write(name, ("-----BEGIN " + kind + "-----\n" + Base64.getEncoder().encodeToString(bytes)
            + "\n-----END " + kind + "-----\n").getBytes(StandardCharsets.US_ASCII));
    }
    private KeyPair keys(String algorithm) throws Exception {
        KeyPairGenerator generator = KeyPairGenerator.getInstance(algorithm);
        if (algorithm.equals("EC")) generator.initialize(new ECGenParameterSpec("secp256r1"));
        else generator.initialize(2048);
        return generator.generateKeyPair();
    }
    private byte[] decodePem(Path path) throws Exception {
        String text = new String(Files.readAllBytes(path), StandardCharsets.US_ASCII);
        return Base64.getDecoder().decode(text.replaceAll("-----[^-]+-----", "").replaceAll("\\s", ""));
    }

    @Test void usageAndUnknownCommandsExitUnsuccessfully() throws Exception {
        cli().failure("Usage:");
        cli("unknown-operation").failure("Unknown command: unknown-operation");
        for (String command : Arrays.asList("create-keys", "encrypt", "decrypt", "sign", "sign-bulk",
                "verify", "sign-ec", "verify-ec", "sign-rsa", "verify-rsa", "public-key")) {
            cli(command).failure("Usage: " + command);
        }
    }

    @Test void createdPemKeysAreUsableByTheIndependentJdkProvider() throws Exception {
        Path pub = dir.resolve("public.pem"), priv = dir.resolve("private.pem");
        cli("create-keys", pub.toString(), priv.toString()).success();
        KeyFactory factory = KeyFactory.getInstance("EC");
        PublicKey publicKey = factory.generatePublic(new X509EncodedKeySpec(decodePem(pub)));
        PrivateKey privateKey = factory.generatePrivate(new PKCS8EncodedKeySpec(decodePem(priv)));
        assertEquals(256, ((java.security.interfaces.ECPublicKey) publicKey).getParams().getCurve().getField().getFieldSize());
        byte[] message = new byte[] {0, 1, -1, 10, 42};
        Path file = write("message.bin", message);
        Signature signer = Signature.getInstance("SHA256withECDSA");
        signer.initSign(privateKey); signer.update(message);
        assertEquals("Signature valid: true", cli("verify", pub.toString(), file.toString(), Base64.getEncoder().encodeToString(signer.sign())).success());
        Signature verifier = Signature.getInstance("SHA256withECDSA");
        verifier.initVerify(publicKey); verifier.update(message);
        assertTrue(verifier.verify(Base64.getDecoder().decode(cli("sign", priv.toString(), file.toString()).success())));
    }

    @Test void verifiesIndependentSignaturesAndRejectsChangedContentForBothAlgorithms() throws Exception {
        for (String algorithm : Arrays.asList("EC", "RSA")) {
            KeyPair pair = keys(algorithm);
            Path pub = pem(algorithm + ".pem", "PUBLIC KEY", pair.getPublic().getEncoded());
            byte[] message = "synthetic ballot\n".getBytes(StandardCharsets.UTF_8);
            Path file = write(algorithm + ".txt", message);
            Signature signer = Signature.getInstance(algorithm.equals("EC") ? "SHA256withECDSA" : "SHA256withRSA");
            signer.initSign(pair.getPrivate()); signer.update(message);
            String signature = Base64.getEncoder().encodeToString(signer.sign());
            String operation = algorithm.equals("EC") ? "verify" : "verify-rsa";
            assertEquals("Signature valid: true", cli(operation, pub.toString(), file.toString(), signature).success());
            Files.write(file, "changed ballot\n".getBytes(StandardCharsets.UTF_8));
            assertEquals("Signature valid: false", cli(operation, pub.toString(), file.toString(), signature).success());
            cli(operation, pub.toString(), file.toString(), "!not-base64!").failure("IllegalArgumentException");
        }
    }

    @Test void encryptedBytesInteroperateAndTamperingFailsAuthentication() throws Exception {
        KeyPair pair = keys("EC");
        Path pub = pem("ec.pub", "PUBLIC KEY", pair.getPublic().getEncoded());
        Path priv = pem("ec.key", "PRIVATE KEY", pair.getPrivate().getEncoded());
        // An explicit provider instance leaves the parent JVM's global registry untouched.
        Provider provider = new BouncyCastleProvider();
        KeyFactory factory = KeyFactory.getInstance("EC", provider);
        IESParameterSpec spec = new IESParameterSpec(null, null, 256, 256, null, false);
        byte[] clear = "literal UTF-8 text: ñ".getBytes(StandardCharsets.UTF_8);
        Cipher reference = Cipher.getInstance("ECIES", provider);
        reference.init(Cipher.DECRYPT_MODE, factory.generatePrivate(new PKCS8EncodedKeySpec(pair.getPrivate().getEncoded())), spec);
        byte[] encrypted = Base64.getDecoder().decode(cli("encrypt", pub.toString(), new String(clear, StandardCharsets.UTF_8)).success());
        assertArrayEquals(clear, reference.doFinal(encrypted));
        reference.init(Cipher.ENCRYPT_MODE, factory.generatePublic(new X509EncodedKeySpec(pair.getPublic().getEncoded())), spec);
        byte[] independent = reference.doFinal(clear);
        assertEquals(new String(clear, StandardCharsets.UTF_8), cli("decrypt", priv.toString(), Base64.getEncoder().encodeToString(independent)).success());
        independent[independent.length - 1] ^= 1;
        cli("decrypt", priv.toString(), Base64.getEncoder().encodeToString(independent)).failure("invalid MAC");
        cli("decrypt", priv.toString(), "!").failure("IllegalArgumentException");
        Path malformed = write("bad.pem", "not a public key".getBytes(StandardCharsets.US_ASCII));
        cli("encrypt", malformed.toString(), "message").failure("IllegalArgumentException");
    }

    @Test void bulkSigningIgnoresDirectoriesAndExistingSignatureFiles() throws Exception {
        KeyPair pair = keys("EC");
        Path priv = pem("private.pem", "PRIVATE KEY", pair.getPrivate().getEncoded());
        Path folder = Files.createDirectory(dir.resolve("documents"));
        byte[] message = new byte[] {1, 0, 2, -1};
        Files.write(folder.resolve("ballot.bin"), message);
        Files.write(folder.resolve("existing.SIGN"), new byte[] {7});
        Files.createDirectory(folder.resolve("nested"));
        cli("sign-bulk", priv.toString(), folder.toString()).success();
        assertArrayEquals(new byte[] {7}, Files.readAllBytes(folder.resolve("existing.SIGN")));
        assertFalse(Files.exists(folder.resolve("existing.SIGN.sign")));
        assertFalse(Files.exists(folder.resolve("nested.sign")));
        Signature verifier = Signature.getInstance("SHA256withECDSA");
        verifier.initVerify(pair.getPublic()); verifier.update(message);
        assertTrue(verifier.verify(Base64.getDecoder().decode(Files.readAllBytes(folder.resolve("ballot.bin.sign")))));
        Path empty = Files.createDirectory(dir.resolve("empty"));
        assertTrue(cli("sign-bulk", priv.toString(), empty.toString()).success().contains("No files found to sign"));
    }
}
