// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package com.example;

import org.spongycastle.jce.ECNamedCurveTable;
import org.spongycastle.jce.spec.ECParameterSpec;
import org.spongycastle.jce.spec.IESParameterSpec;
import org.spongycastle.jce.provider.BouncyCastleProvider;

import javax.crypto.Cipher;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.security.*;
import java.security.interfaces.ECPrivateKey;
import java.security.interfaces.ECPublicKey;
import java.security.spec.ECGenParameterSpec;
import java.security.spec.PKCS8EncodedKeySpec;
import java.security.spec.X509EncodedKeySpec;
import java.util.Base64;
import java.io.StringWriter;
import java.io.PrintWriter;

public class ECIESEncryptionTool {

    public static void main(String[] args) throws Exception {
        Security.addProvider(new BouncyCastleProvider());

        if (args.length < 1) {
            System.out.println("Usage:");
            System.out.println("  create-keys <public-key-file> <private-key-file>");
            System.out.println("  encrypt <public-key-file> <plaintext-base64>");
            System.out.println("  decrypt <private-key-file> <encrypted-text>");
            System.out.println("  sign <private-key-file> <plaintext-file>");
            System.out.println("  verify <public-key-file> <plaintext-file> <signature-base64>");
            return;
        }

        String command = args[0];
        switch (command) {
            case "create-keys":
                if (args.length != 3) {
                    System.out.println("Usage: create-keys <public-key-file> <private-key-file>");
                    return;
                }
                createKeys(args[1], args[2]);
                break;

            case "encrypt":
                if (args.length != 3) {
                    System.out.println("Usage: encrypt <public-key-file> <plaintext-base64>");
                    return;
                }
                String encryptedText = encryptText(args[1], args[2]);
                System.out.println(encryptedText);
                break;

            case "decrypt":
                if (args.length != 3) {
                    System.out.println("Usage: decrypt <private-key-file> <encrypted-text>");
                    return;
                }
                String decryptedText = decryptText(args[1], args[2]);
                System.out.println(decryptedText);
                break;

            case "sign":
                if (args.length != 3) {
                    System.out.println("Usage: sign <private-key-file> <plaintext-file>");
                    return;
                }
                String signature = signText(args[1], args[2]);
                System.out.println(signature);
                break;

            case "verify":
                if (args.length != 4) {
                    System.out.println("Usage: verify <public-key-file> <plaintext-file> <signature-base64>");
                    return;
                }
                boolean isValid = verifyText(args[1], args[2], args[3]);
                System.out.println("Signature valid: " + isValid);
                break;

            default:
                System.out.println("Unknown command: " + command);
                break;
        }
    }

    private static void createKeys(String publicKeyFile, String privateKeyFile) throws Exception {
        KeyPairGenerator keyGen = KeyPairGenerator.getInstance("EC", "SC");
        keyGen.initialize(new ECGenParameterSpec("secp256r1"), new SecureRandom());
        KeyPair keyPair = keyGen.generateKeyPair();

        String publicKeyPEM = getPublicKeyPEM(keyPair.getPublic());
        String privateKeyPEM = getPrivateKeyPEM(keyPair.getPrivate());

        writeFile(publicKeyFile, publicKeyPEM);
        writeFile(privateKeyFile, privateKeyPEM);

        System.out.println("Keys created and saved to files:");
        System.out.println("  Public key: " + publicKeyFile);
        System.out.println("  Private key: " + privateKeyFile);
    }

    private static String encryptText(String publicKeyFile, String plaintext) throws Exception {
        PublicKey publicKey = loadPublicKeyFromPEM(readFile(publicKeyFile));

        // Initialize the Cipher for encryption
        Cipher iesCipher = Cipher.getInstance("ECIES", "SC");
        IESParameterSpec spec = new IESParameterSpec(
                null,  // No derivation
                null,  // No encoding
                256,   // MAC key size in bits
                256,   // Cipher key size in bits
                null,  // No nonce
                false  // Use point compression
        );
        iesCipher.init(Cipher.ENCRYPT_MODE, publicKey, spec);

        // Encrypt the plaintext
        byte[] ciphertext = iesCipher.doFinal(plaintext.getBytes());

        return Base64.getEncoder().encodeToString(ciphertext);
    }

    private static String decryptText(String privateKeyFile, String encryptedTextBase64) throws Exception {
        PrivateKey privateKey = loadPrivateKeyFromPEM(readFile(privateKeyFile));

        // Decode the Base64-encoded encrypted text to get the original byte array
        byte[] encryptedTextBytes = Base64.getDecoder().decode(encryptedTextBase64);

        // Initialize the Cipher for decryption
        Cipher iesCipher = Cipher.getInstance("ECIES", "SC");
        iesCipher.init(Cipher.DECRYPT_MODE, privateKey, new SecureRandom());

        // Decrypt the ciphertext
        byte[] decryptedTextBytes = iesCipher.doFinal(encryptedTextBytes);

        return Base64.getEncoder().encodeToString(decryptedTextBytes);
    }

    private static String signText(String privateKeyFile, String plaintextFilePath) throws Exception {
        PrivateKey privateKey = loadPrivateKeyFromPEM(readFile(privateKeyFile));
    
        // Read the plaintext from the file to get the original byte array
        byte[] plaintextBytes = Files.readAllBytes(Paths.get(plaintextFilePath));
    
        // Initialize the Signature object for signing
        Signature signature = Signature.getInstance("SHA256withECDSA", "SC");
        signature.initSign(privateKey);
        signature.update(plaintextBytes);
    
        // Sign the plaintext
        byte[] signatureBytes = signature.sign();
    
        return Base64.getEncoder().encodeToString(signatureBytes);
    }
    

    private static boolean verifyText(String publicKeyFile, String plaintextFilePath, String signatureBase64) throws Exception {
        PublicKey publicKey = loadPublicKeyFromPEM(readFile(publicKeyFile));
    
        // Read the plaintext from the file to get the original byte array
        byte[] plaintextBytes = Files.readAllBytes(Paths.get(plaintextFilePath));
    
        // Decode the Base64-encoded signature to get the original byte array
        byte[] signatureBytes = Base64.getDecoder().decode(signatureBase64);
    
        // Initialize the Signature object for verification
        Signature signature = Signature.getInstance("SHA256withECDSA", "SC");
        signature.initVerify(publicKey);
        signature.update(plaintextBytes);
    
        // Verify the signature
        return signature.verify(signatureBytes);
    }

    private static void writeFile(String path, String content) throws IOException {
        try (FileWriter writer = new FileWriter(new File(path))) {
            writer.write(content);
        }
    }

    private static String readFile(String path) throws IOException {
        return new String(Files.readAllBytes(Paths.get(path)));
    }

    private static String getPublicKeyPEM(PublicKey publicKey) throws Exception {
        X509EncodedKeySpec x509EncodedKeySpec = new X509EncodedKeySpec(publicKey.getEncoded());
        StringWriter stringWriter = new StringWriter();
        try (PrintWriter writer = new PrintWriter(stringWriter)) {
            writer.println("-----BEGIN PUBLIC KEY-----");
            writer.println(Base64.getMimeEncoder(64, new byte[]{'\n'}).encodeToString(x509EncodedKeySpec.getEncoded()));
            writer.println("-----END PUBLIC KEY-----");
        }
        return stringWriter.toString();
    }

    private static String getPrivateKeyPEM(PrivateKey privateKey) throws Exception {
        PKCS8EncodedKeySpec pkcs8EncodedKeySpec = new PKCS8EncodedKeySpec(privateKey.getEncoded());
        StringWriter stringWriter = new StringWriter();
        try (PrintWriter writer = new PrintWriter(stringWriter)) {
            writer.println("-----BEGIN PRIVATE KEY-----");
            writer.println(Base64.getMimeEncoder(64, new byte[]{'\n'}).encodeToString(pkcs8EncodedKeySpec.getEncoded()));
            writer.println("-----END PRIVATE KEY-----");
        }
        return stringWriter.toString();
    }

    private static PublicKey loadPublicKeyFromPEM(String pem) throws Exception {
        String publicKeyPEM = pem.replace("-----BEGIN PUBLIC KEY-----", "")
                                 .replace("-----END PUBLIC KEY-----", "")
                                 .replaceAll("\\s", "");
        byte[] decoded = Base64.getDecoder().decode(publicKeyPEM);
        X509EncodedKeySpec spec = new X509EncodedKeySpec(decoded);
        KeyFactory keyFactory = KeyFactory.getInstance("EC", "SC");
        return keyFactory.generatePublic(spec);
    }

    private static PrivateKey loadPrivateKeyFromPEM(String pem) throws Exception {
        String privateKeyPEM = pem.replace("-----BEGIN PRIVATE KEY-----", "")
                                  .replace("-----END PRIVATE KEY-----", "")
                                  .replaceAll("\\s", "");
        byte[] decoded = Base64.getDecoder().decode(privateKeyPEM);
        PKCS8EncodedKeySpec spec = new PKCS8EncodedKeySpec(decoded);
        KeyFactory keyFactory = KeyFactory.getInstance("EC", "SC");
        return keyFactory.generatePrivate(spec);
    }
}
