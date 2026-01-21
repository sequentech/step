// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package com.example;

import org.spongycastle.jce.ECNamedCurveTable;
import org.spongycastle.jce.spec.ECParameterSpec;
import org.spongycastle.jce.spec.IESParameterSpec;
import org.spongycastle.jce.provider.BouncyCastleProvider;

import javax.crypto.Cipher;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.security.*;
import java.security.interfaces.ECPrivateKey;
import java.security.interfaces.ECPublicKey;
import java.security.interfaces.RSAPrivateKey;
import java.security.spec.ECGenParameterSpec;
import java.security.spec.PKCS8EncodedKeySpec;
import java.security.spec.X509EncodedKeySpec;
import java.util.Base64;
import java.io.StringWriter;
import java.io.PrintWriter;
import java.security.cert.*;
import java.security.interfaces.RSAPublicKey;

public class ECIESEncryptionTool {

    public static void main(String[] args) throws Exception {
        Security.addProvider(new BouncyCastleProvider());

        if (args.length < 1) {
            System.out.println("Usage:");
            System.out.println("  create-keys <public-key-file> <private-key-file>");
            System.out.println("  encrypt <public-key-file> <plaintext-base64>");
            System.out.println("  decrypt <private-key-file> <encrypted-text>");
            System.out.println("  sign <private-key-file> <plaintext-file>");
            System.out.println("  sign-bulk <private-key-file> <folder>");
            System.out.println("  verify <public-key-file> <plaintext-file> <signature-base64>");
            System.out.println("  sign-ec <p12-private-key-file> <plaintext-file> <p12-password>");
            System.out.println("  verify-ec <cert-file> <plaintext-file> <signature-base64> (<ca-cert-file>)");
            System.out.println("  sign-rsa <p12-private-key-file> <plaintext-file> <p12-password>");
            System.out.println("  verify-rsa <public-key-file> <plaintext-file> <signature-base64>");
            System.out.println("  public-key <p12-private-key-file> <p12-password>");
            System.exit(1);

            return;
        }

        String command = args[0];
        switch (command) {
            case "create-keys":
                if (args.length != 3) {
                    System.out.println("Usage: create-keys <public-key-file> <private-key-file>");
                    System.exit(1);
                    return;
                }
                createKeys(args[1], args[2]);
                break;

            case "encrypt":
                if (args.length != 3) {
                    System.out.println("Usage: encrypt <public-key-file> <plaintext-base64>");
                    System.exit(1);
                    return;
                }
                String encryptedText = encryptText(args[1], args[2]);
                System.out.println(encryptedText);
                break;

            case "decrypt":
                if (args.length != 3) {
                    System.out.println("Usage: decrypt <private-key-file> <encrypted-text>");
                    System.exit(1);
                    return;
                }
                String decryptedText = decryptText(args[1], args[2]);
                System.out.println(decryptedText);
                break;

            case "sign":
                if (args.length != 3) {
                    System.out.println("Usage: sign <private-key-file> <plaintext-file>");
                    System.exit(1);
                    return;
                }
                String signature = signText(args[1], args[2], true);
                System.out.println(signature);
                break;

            case "sign-bulk":
                if (args.length != 3) {
                    System.out.println("Usage: sign-bulk <private-key-file> <folder>");
                    System.exit(1);
                    return;
                }
                signBulk(args[1], args[2], true);
                break;

            case "verify":
                if (args.length != 4) {
                    System.out.println("Usage: verify <public-key-file> <plaintext-file> <signature-base64>");
                    System.exit(1);
                    return;
                }
                boolean isValid = verifyText(args[1], args[2], args[3], true);
                System.out.println("Signature valid: " + isValid);
                break;

            case "sign-ec":
                if (args.length != 4) {
                    System.out.println("Usage: sign-ec <p12-private-key-file> <plaintext-file> <p12-password>");
                    System.exit(1);
                    return;
                }
                String ecSignature = signTextP12(args[1], args[2], true, args[3]);
                System.out.println(ecSignature);
                break;

            case "verify-ec":
                if (args.length != 4 && args.length != 5) {
                    System.out.println("Usage: verify-ec <cert-file> <plaintext-file> <signature-base64> (<ca-cert-file>)");
                    System.exit(1);
                    return;
                }
                if (args.length == 4) {
                    boolean isValidEC = fullVerifyText(args[1], args[2], args[3], null, true);
                    System.out.println("Signature valid: " + isValidEC);
                } else {
                    boolean isValidEC = fullVerifyText(args[1], args[2], args[3], args[4], true);
                    System.out.println("Signature valid: " + isValidEC);
                }
                break;

            case "sign-rsa":
                if (args.length != 4) {
                    System.out.println("Usage: sign-rsa <p12-private-key-file> <plaintext-file> <p12-password>");
                    System.exit(1);
                    return;
                }
                String rsaSignature = signTextP12(args[1], args[2], false, args[3]);
                System.out.println(rsaSignature);
                break;

            case "verify-rsa":
                if (args.length != 4) {
                    System.out.println("Usage: verify-rsa <public-key-file> <plaintext-file> <signature-base64>");
                    System.exit(1);
                    return;
                }
                boolean isValidRsa = verifyText(args[1], args[2], args[3], false);
                System.out.println("Signature valid: " + isValidRsa);
                break;

            case "public-key":
                if (args.length != 3) {
                    System.out.println("Usage: public-key <p12-private-key-file> <p12-password>");
                    System.exit(1);
                    return;
                }
                String publicKey = publicKeyPemFromP12(args[1], args[2]);
                System.out.println(publicKey);
                break;

            default:
                System.out.println("Unknown command: " + command);
                System.exit(1);
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
        IESParameterSpec spec = new IESParameterSpec(
                null,  // No derivation
                null,  // No encoding
                256,   // MAC key size in bits
                256,   // Cipher key size in bits
                null,  // No nonce
                false  // Use point compression
        );
        iesCipher.init(Cipher.DECRYPT_MODE, privateKey, spec);

        // Decrypt the ciphertext
        byte[] decryptedTextBytes = iesCipher.doFinal(encryptedTextBytes);

        return new String(decryptedTextBytes, StandardCharsets.UTF_8);
    }

    private static String signText(String privateKeyFile, String plaintextFilePath, Boolean isECDSA) throws Exception {
        String algorithm = isECDSA? "SHA256withECDSA" : "SHA256withRSA";
        PrivateKey privateKey = loadPrivateKeyFromPEM(readFile(privateKeyFile));
    
        // Read the plaintext from the file to get the original byte array
        byte[] plaintextBytes = Files.readAllBytes(Paths.get(plaintextFilePath));
    
        // Initialize the Signature object for signing
        Signature signature = Signature.getInstance(algorithm, "SC");
        signature.initSign(privateKey);
        signature.update(plaintextBytes);
    
        // Sign the plaintext
        byte[] signatureBytes = signature.sign();
    
        return Base64.getEncoder().encodeToString(signatureBytes);
    }

    private static void signBulk(String privateKeyFile, String folderPath, Boolean isECDSA) throws Exception {
        String algorithm = isECDSA ? "SHA256withECDSA" : "SHA256withRSA";
    
        // Load the private key from PEM
        PrivateKey privateKey = loadPrivateKeyFromPEM(readFile(privateKeyFile));
    
        // Validate that folderPath is indeed a directory
        File folder = new File(folderPath);
        if (!folder.isDirectory()) {
            System.err.println("Error: " + folderPath + " is not a directory or does not exist.");
            return;
        }
    
        // Find all files that do NOT end with ".sign"
        // Also skip subdirectories
        File[] inputFiles = folder.listFiles(file -> 
            file.isFile() && !file.getName().toLowerCase().endsWith(".sign")
        );
    
        if (inputFiles == null || inputFiles.length == 0) {
            System.out.println("No files found to sign in: " + folderPath);
            return;
        }
    
        for (File inputFile : inputFiles) {
            // Read file bytes
            byte[] fileContents = Files.readAllBytes(inputFile.toPath());
    
            // Initialize and create the signature
            Signature signature = Signature.getInstance(algorithm, "SC");
            signature.initSign(privateKey);
            signature.update(fileContents);
            byte[] signatureBytes = signature.sign();
    
            // Convert to Base64
            String signatureBase64 = Base64.getEncoder().encodeToString(signatureBytes);
    
            // Create a .sign file next to the original file
            // e.g. "message.eml" -> "message.eml.sign"
            // or "invoice.pdf" -> "invoice.pdf.sign"
            String signFilePath = inputFile.getAbsolutePath() + ".sign";
            writeFile(signFilePath, signatureBase64);
        }
    }

    private static String signTextP12(String p12FilePath, String plaintextFilePath, Boolean isECDSA, String password) throws Exception {
        String algorithm = isECDSA ? "SHA256withECDSA" : "SHA256withRSA";
        
        // Load the P12 file and extract the private key
        KeyStore keyStore = KeyStore.getInstance("PKCS12");
        try (FileInputStream fis = new FileInputStream(p12FilePath)) {
            keyStore.load(fis, password.toCharArray());
        }
        
        // Assuming the alias is the first one in the keystore
        String alias = keyStore.aliases().nextElement();
        PrivateKey privateKey = (PrivateKey) keyStore.getKey(alias, password.toCharArray());

        // Read the plaintext from the file to get the original byte array
        byte[] plaintextBytes = Files.readAllBytes(Paths.get(plaintextFilePath));

        // Initialize the Signature object for signing
        Signature signature = Signature.getInstance(algorithm, "SC");
        signature.initSign(privateKey);
        signature.update(plaintextBytes);

        // Sign the plaintext
        byte[] signatureBytes = signature.sign();

        return Base64.getEncoder().encodeToString(signatureBytes);
    }

    private static PublicKey derivePublicKeyFromPrivateKey(PrivateKey privateKey) throws Exception {
        if (privateKey instanceof ECPrivateKey) {
            // Handle ECPrivateKey (Elliptic Curve key)
            ECPrivateKey ecPrivateKey = (ECPrivateKey) privateKey;
            org.spongycastle.jce.spec.ECParameterSpec ecSpec = ECNamedCurveTable.getParameterSpec("secp256r1");
    
            // Convert the private key's scalar (d) into a BigInteger
            java.math.BigInteger d = ecPrivateKey.getS();
    
            // Generate the public key point using the private key's scalar (d)
            org.spongycastle.math.ec.ECPoint q = ecSpec.getG().multiply(d).normalize();
    
            // Create the public key spec using Bouncy Castle's ECPublicKeySpec
            org.spongycastle.jce.spec.ECPublicKeySpec publicKeySpec = new org.spongycastle.jce.spec.ECPublicKeySpec(q, ecSpec);
    
            // Create the key factory and generate the public key using Bouncy Castle
            KeyFactory keyFactory = KeyFactory.getInstance("EC", "SC");
            return keyFactory.generatePublic(publicKeySpec);
    
        } else if (privateKey instanceof RSAPrivateKey) {
            // Handle RSAPrivateKey (RSA key)
            KeyFactory keyFactory = KeyFactory.getInstance("RSA");
            RSAPrivateKey rsaPrivateKey = (RSAPrivateKey) privateKey;
    
            // Generate the public key from the RSA private key modulus and public exponent
            java.security.spec.RSAPublicKeySpec publicKeySpec = new java.security.spec.RSAPublicKeySpec(
                rsaPrivateKey.getModulus(), java.math.BigInteger.valueOf(65537)); // 65537 is the common public exponent
    
            return keyFactory.generatePublic(publicKeySpec);
    
        } else {
            throw new IllegalArgumentException("Unsupported private key type.");
        }
    }

    private static String publicKeyPemFromP12(String p12FilePath, String password)  throws Exception {
        // Load the P12 file and extract the private key
        KeyStore keyStore = KeyStore.getInstance("PKCS12");
        try (FileInputStream fis = new FileInputStream(p12FilePath)) {
            keyStore.load(fis, password.toCharArray());
        }
        
        // Assuming the alias is the first one in the keystore
        String alias = keyStore.aliases().nextElement();
        PrivateKey privateKey = (PrivateKey) keyStore.getKey(alias, password.toCharArray());
        PublicKey publicKey = derivePublicKeyFromPrivateKey(privateKey);

        // Convert the PublicKey to PEM format
        return getPublicKeyPEM(publicKey);
    }

    private static boolean fullVerifyText(
        String certificateFile,        // The certificate file of the signer (instead of a raw public key)
        String plaintextFilePath,
        String signatureBase64,
        String caCertificateFile,      // The CA certificate file
        Boolean isECDSA
    ) throws Exception {
        String algorithm = isECDSA ? "SHA256withECDSA" : "SHA256withRSA";

        // Load the signer's certificate
        CertificateFactory cf = CertificateFactory.getInstance("X.509");
        X509Certificate signerCert;
        try (java.io.FileInputStream certFis = new java.io.FileInputStream(certificateFile)) {
            signerCert = (X509Certificate) cf.generateCertificate(certFis);
        }

        // 1. Check if the signer's certificate is valid (not expired)
        signerCert.checkValidity();

        if (null != caCertificateFile && !caCertificateFile.isEmpty()) {
            // Load the CA certificate
            X509Certificate caCert;
            try (java.io.FileInputStream caFis = new java.io.FileInputStream(caCertificateFile)) {
                caCert = (X509Certificate) cf.generateCertificate(caFis);
            }

            // 2. Check that the certificate was issued by the given CA
            //    Compare the issuer of the signer's certificate to the subject of the CA certificate
            if (!signerCert.getIssuerX500Principal().equals(caCert.getSubjectX500Principal())) {
                throw new SecurityException("The certificate was not issued by the specified CA.");
            }

            // 3. Verify the certificate's signature using the CA's public key
            try {
                signerCert.verify(caCert.getPublicKey());
            } catch (SignatureException | InvalidKeyException | CertificateException | NoSuchAlgorithmException | NoSuchProviderException e) {
                throw new SecurityException("Failed to verify certificate with the provided CA public key.", e);
            }
        }

        // If we reach here, it means:
        // - The certificate is not expired
        // - The certificate issuer matches the CA's subject
        // - The certificate signature is verified against the CA certificate's public key

        // Now, proceed to verify the actual signature over the plaintext
        PublicKey publicKey = signerCert.getPublicKey();

        // Read the plaintext from the file
        byte[] plaintextBytes = Files.readAllBytes(Paths.get(plaintextFilePath));

        // Decode the Base64-encoded signature
        byte[] signatureBytes = Base64.getDecoder().decode(signatureBase64);

        // Initialize the Signature object for verification
        // Use the provider if needed, e.g., "SC". If default provider supports the algorithm, "SC" may not be required.
        Signature signature = Signature.getInstance(algorithm, "SC");
        signature.initVerify(publicKey);
        signature.update(plaintextBytes);

        // Verify the signature
        return signature.verify(signatureBytes);
    }

    private static boolean verifyText(String publicKeyFile, String plaintextFilePath, String signatureBase64, Boolean isECDSA) throws Exception {
        String algorithm = isECDSA? "SHA256withECDSA" : "SHA256withRSA";
        PublicKey publicKey = loadPublicKeyFromPEM(readFile(publicKeyFile));
    
        // Read the plaintext from the file to get the original byte array
        byte[] plaintextBytes = Files.readAllBytes(Paths.get(plaintextFilePath));
    
        // Decode the Base64-encoded signature to get the original byte array
        byte[] signatureBytes = Base64.getDecoder().decode(signatureBase64);
    
        // Initialize the Signature object for verification
        Signature signature = Signature.getInstance(algorithm, "SC");
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
