package com.example;

import org.bouncycastle.jce.ECNamedCurveTable;
import org.bouncycastle.jce.spec.ECParameterSpec;
import org.bouncycastle.crypto.engines.IESEngine;
import org.bouncycastle.crypto.params.*;
import org.bouncycastle.crypto.digests.SHA1Digest;
import org.bouncycastle.crypto.generators.KDF2BytesGenerator;
import org.bouncycastle.crypto.macs.HMac;
import org.bouncycastle.crypto.engines.AESEngine;
import org.bouncycastle.crypto.modes.CBCBlockCipher;
import org.bouncycastle.crypto.paddings.PaddedBufferedBlockCipher;
import org.bouncycastle.crypto.agreement.ECDHBasicAgreement;
import org.bouncycastle.jce.provider.BouncyCastleProvider;
import org.bouncycastle.math.ec.ECPoint;

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.security.*;
import java.security.interfaces.ECPrivateKey;
import java.security.interfaces.ECPublicKey;
import java.security.spec.ECGenParameterSpec;  // <-- Add this import
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
                System.out.println("Encrypted text: " + encryptedText);
                break;

            case "decrypt":
                if (args.length != 3) {
                    System.out.println("Usage: decrypt <private-key-file> <encrypted-text>");
                    return;
                }
                String decryptedText = decryptText(args[1], args[2]);
                System.out.println("Decrypted text: " + decryptedText);
                break;

            default:
                System.out.println("Unknown command: " + command);
                break;
        }
    }

    private static void createKeys(String publicKeyFile, String privateKeyFile) throws Exception {
        KeyPairGenerator keyGen = KeyPairGenerator.getInstance("EC", "BC");
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
        KeyPairGenerator keyGen = KeyPairGenerator.getInstance("EC", "BC");
        keyGen.initialize(new ECGenParameterSpec("secp256r1"), new SecureRandom());
        KeyPair ephemeralKeyPair = keyGen.generateKeyPair();
    
        ECParameterSpec ecSpec = ECNamedCurveTable.getParameterSpec("P-256");
        ECDomainParameters domainParams = new ECDomainParameters(
            ecSpec.getCurve(),
            ecSpec.getG(),
            ecSpec.getN(),
            ecSpec.getH()
        );
    
        // Convert the recipient's public key to BouncyCastle compatible parameters
        ECPublicKey javaPublicKey = (ECPublicKey) publicKey;
        org.bouncycastle.math.ec.ECPoint bcPublicPoint = ecSpec.getCurve().createPoint(
            javaPublicKey.getW().getAffineX(),
            javaPublicKey.getW().getAffineY()
        );
        ECPublicKeyParameters publicKeyParams = new ECPublicKeyParameters(bcPublicPoint, domainParams);
    
        // Convert the sender's ephemeral private key to BouncyCastle compatible parameters
        ECPrivateKey javaPrivateKey = (ECPrivateKey) ephemeralKeyPair.getPrivate();
        ECPrivateKeyParameters privateKeyParams = new ECPrivateKeyParameters(javaPrivateKey.getS(), domainParams);
    
        // Set up IESEngine with ECDH, KDF2 (SHA-1), and AES-128-CBC with padding
        IESEngine iesEngine = new IESEngine(
                new ECDHBasicAgreement(),
                new KDF2BytesGenerator(new SHA1Digest()),
                new HMac(new SHA1Digest()),
                new PaddedBufferedBlockCipher(new CBCBlockCipher(new AESEngine()))
        );
    
        IESWithCipherParameters params = new IESWithCipherParameters(null, null, 128, 128);
        iesEngine.init(true, privateKeyParams, publicKeyParams, params);
    
        // Encrypt the plaintext
        byte[] ciphertext = iesEngine.processBlock(plaintext.getBytes(), 0, plaintext.getBytes().length);
    
        // Get the raw ephemeral public key point (as a Bouncy Castle ECPoint)
        ECPublicKey ephemeralPublicKey = (ECPublicKey) ephemeralKeyPair.getPublic();
        org.bouncycastle.math.ec.ECPoint bcEphemeralPoint = ecSpec.getCurve().createPoint(
            ephemeralPublicKey.getW().getAffineX(),
            ephemeralPublicKey.getW().getAffineY()
        );
        byte[] ephemeralPublicKeyEncoded = bcEphemeralPoint.getEncoded(false);
    
        // Combine the ephemeral public key point with the ciphertext
        byte[] finalCiphertext = new byte[ephemeralPublicKeyEncoded.length + ciphertext.length];
        System.arraycopy(ephemeralPublicKeyEncoded, 0, finalCiphertext, 0, ephemeralPublicKeyEncoded.length);
        System.arraycopy(ciphertext, 0, finalCiphertext, ephemeralPublicKeyEncoded.length, ciphertext.length);
    
        return Base64.getEncoder().encodeToString(finalCiphertext);
    }

    private static String decryptText(String privateKeyFile, String encryptedText) throws Exception {
        // Load the recipient's private key from the PEM file
        PrivateKey privateKey = loadPrivateKeyFromPEM(readFile(privateKeyFile));
    
        // Convert the recipient's private key to BouncyCastle compatible parameters
        ECParameterSpec ecSpec = ECNamedCurveTable.getParameterSpec("P-256");
        ECDomainParameters domainParams = new ECDomainParameters(
            ecSpec.getCurve(),
            ecSpec.getG(),
            ecSpec.getN(),
            ecSpec.getH()
        );
        
        ECPrivateKey javaPrivateKey = (ECPrivateKey) privateKey;
        ECPrivateKeyParameters privateKeyParams = new ECPrivateKeyParameters(javaPrivateKey.getS(), domainParams);
    
        // Decode the full ciphertext (which includes the ephemeral public key)
        byte[] decodedText = Base64.getDecoder().decode(encryptedText);
    
        // Determine the length of the ephemeral public key (this should be done correctly)
        int ephemeralKeyLength = ecSpec.getCurve().getFieldSize() / 8 * 2 + 1;
        byte[] ephemeralPublicKeyBytes = new byte[ephemeralKeyLength];
        System.arraycopy(decodedText, 0, ephemeralPublicKeyBytes, 0, ephemeralKeyLength);
    
        // Extract the actual ciphertext (after the ephemeral key)
        byte[] ciphertextOnly = new byte[decodedText.length - ephemeralKeyLength];
        System.arraycopy(decodedText, ephemeralKeyLength, ciphertextOnly, 0, ciphertextOnly.length);
    
        // Decode the ephemeral public key
        org.bouncycastle.math.ec.ECPoint bcEphemeralPoint = ecSpec.getCurve().decodePoint(ephemeralPublicKeyBytes);
        ECPublicKeyParameters ephemeralPublicKeyParams = new ECPublicKeyParameters(bcEphemeralPoint, domainParams);
    
        // Initialize the IESEngine for decryption
        IESEngine iesEngine = new IESEngine(
                new ECDHBasicAgreement(),
                new KDF2BytesGenerator(new SHA1Digest()),
                new HMac(new SHA1Digest()),
                new PaddedBufferedBlockCipher(new CBCBlockCipher(new AESEngine()))
        );
    
        IESWithCipherParameters params = new IESWithCipherParameters(null, null, 128, 128);
        iesEngine.init(false, privateKeyParams, ephemeralPublicKeyParams, params);
    
        // Decrypt the ciphertext
        byte[] decryptedText = iesEngine.processBlock(ciphertextOnly, 0, ciphertextOnly.length);
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

    // Method to convert PublicKey to PEM format
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

    // Method to convert PrivateKey to PEM format
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

    // Method to load a PublicKey from a PEM string
    private static PublicKey loadPublicKeyFromPEM(String pem) throws Exception {
        String publicKeyPEM = pem.replace("-----BEGIN PUBLIC KEY-----", "")
                                 .replace("-----END PUBLIC KEY-----", "")
                                 .replaceAll("\\s", "");
        byte[] decoded = Base64.getDecoder().decode(publicKeyPEM);
        X509EncodedKeySpec spec = new X509EncodedKeySpec(decoded);
        KeyFactory keyFactory = KeyFactory.getInstance("EC", "BC");
        return keyFactory.generatePublic(spec);
    }

    // Method to load a PrivateKey from a PEM string
    private static PrivateKey loadPrivateKeyFromPEM(String pem) throws Exception {
        String privateKeyPEM = pem.replace("-----BEGIN PRIVATE KEY-----", "")
                                  .replace("-----END PRIVATE KEY-----", "")
                                  .replaceAll("\\s", "");
        byte[] decoded = Base64.getDecoder().decode(privateKeyPEM);
        PKCS8EncodedKeySpec spec = new PKCS8EncodedKeySpec(decoded);
        KeyFactory keyFactory = KeyFactory.getInstance("EC", "BC");
        return keyFactory.generatePrivate(spec);
    }
}
