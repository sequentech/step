package com.example;

import org.spongycastle.jce.ECNamedCurveTable;
import org.spongycastle.jce.spec.ECParameterSpec;
import org.spongycastle.crypto.engines.IESEngine;
import org.spongycastle.crypto.params.*;
import org.spongycastle.crypto.digests.SHA1Digest;
import org.spongycastle.crypto.generators.KDF2BytesGenerator;
import org.spongycastle.crypto.macs.HMac;
import org.spongycastle.crypto.engines.AESEngine;
import org.spongycastle.crypto.modes.CBCBlockCipher;
import org.spongycastle.crypto.paddings.PaddedBufferedBlockCipher;
import org.spongycastle.crypto.agreement.ECDHBasicAgreement;
import org.spongycastle.jce.provider.BouncyCastleProvider;
import org.spongycastle.math.ec.ECPoint;
import org.spongycastle.jce.spec.IESParameterSpec;

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
import java.math.BigInteger;
import java.io.StringWriter;
import java.io.PrintWriter;

public class ECIESEncryptionTool {

    public static void main(String[] args) throws Exception {
        Security.addProvider(new BouncyCastleProvider());

        if (args.length < 1) {
            System.out.println("Usage:");
            System.out.println("  create-keys <public-key-file> <private-key-file>");
            System.out.println("  encrypt <public-key-file> <plaintext>");
            System.out.println("  decrypt <private-key-file> <encrypted-text>");
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
                    System.out.println("Usage: encrypt <public-key-file> <plaintext>");
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

        // Generate an ephemeral key pair for the sender
        KeyPairGenerator keyGen = KeyPairGenerator.getInstance("EC", "SC");
        keyGen.initialize(new ECGenParameterSpec("secp256r1"), new SecureRandom());
        KeyPair ephemeralKeyPair = keyGen.generateKeyPair();

        // Set up IESParameterSpec with the specified parameters
        IESParameterSpec iesParams = new IESParameterSpec(
            null,           // derivation
            null,           // encoding
            256,            // macKeySize in bits
            256,            // cipherKeySize in bits
            null,           // nonce
            false           // usePointCompression
        );

        // Initialize the Cipher for encryption
        Cipher iesCipher = Cipher.getInstance("ECIES", "SC");
        iesCipher.init(Cipher.ENCRYPT_MODE, publicKey, iesParams, new SecureRandom());

        // Encrypt the plaintext
        byte[] ciphertext = iesCipher.doFinal(plaintext.getBytes());

        return Base64.getEncoder().encodeToString(ciphertext);
    }

    private static String decryptText(String privateKeyFile, String encryptedText) throws Exception {
        PrivateKey privateKey = loadPrivateKeyFromPEM(readFile(privateKeyFile));

        // Set up IESParameterSpec with the specified parameters
        IESParameterSpec iesParams = new IESParameterSpec(
            null,           // derivation
            null,           // encoding
            256,            // macKeySize in bits
            256,            // cipherKeySize in bits
            null,           // nonce
            false           // usePointCompression
        );

        // Initialize the Cipher for decryption
        Cipher iesCipher = Cipher.getInstance("ECIES", "SC");
        iesCipher.init(Cipher.DECRYPT_MODE, privateKey, iesParams);

        // Decode and decrypt the ciphertext
        byte[] decodedText = Base64.getDecoder().decode(encryptedText);
        byte[] decryptedText = iesCipher.doFinal(decodedText);

        return new String(decryptedText);
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
