package com.example;

import org.bouncycastle.jce.ECNamedCurveTable;
import org.bouncycastle.jce.spec.ECParameterSpec;
import org.bouncycastle.crypto.engines.IESEngine;
import org.bouncycastle.crypto.params.ECDomainParameters;
import org.bouncycastle.crypto.params.ECPrivateKeyParameters;
import org.bouncycastle.crypto.params.ECPublicKeyParameters;
import org.bouncycastle.crypto.params.IESWithCipherParameters;
import org.bouncycastle.crypto.digests.SHA256Digest;
import org.bouncycastle.crypto.macs.HMac;
import org.bouncycastle.math.ec.ECPoint;

import java.nio.file.Files;
import java.nio.file.Paths;
import java.security.KeyFactory;
import java.security.PublicKey;
import java.security.Security;
import java.security.spec.X509EncodedKeySpec;
import java.util.Base64;

public class ECIESEncryptionExample {

    public static void main(String[] args) throws Exception {
        if (args.length < 2) {
            System.out.println("Usage: java ECIESEncryptionExample <public_key.pem> <text_to_encrypt>");
            return;
        }

        String publicKeyPath = args[0];
        String textToEncrypt = args[1];

        Security.addProvider(new org.bouncycastle.jce.provider.BouncyCastleProvider());

        // Read the public key from the PEM file
        PublicKey publicKey = loadPublicKey(publicKeyPath);

        // Encrypt the provided text
        byte[] plaintext = textToEncrypt.getBytes();
        byte[] ciphertext = encrypt(publicKey, plaintext);

        // Print the ciphertext in Base64 format
        System.out.println("Ciphertext: " + Base64.getEncoder().encodeToString(ciphertext));
    }

    public static PublicKey loadPublicKey(String filepath) throws Exception {
        byte[] keyBytes = Files.readAllBytes(Paths.get(filepath));
        String publicKeyPEM = new String(keyBytes)
                .replace("-----BEGIN PUBLIC KEY-----", "")
                .replace("-----END PUBLIC KEY-----", "")
                .replaceAll("\\s", "");

        byte[] decoded = Base64.getDecoder().decode(publicKeyPEM);
        X509EncodedKeySpec spec = new X509EncodedKeySpec(decoded);
        KeyFactory keyFactory = KeyFactory.getInstance("EC", "BC");
        return keyFactory.generatePublic(spec);
    }

    public static byte[] encrypt(PublicKey publicKey, byte[] plaintext) throws Exception {
        ECParameterSpec ecSpec = ECNamedCurveTable.getParameterSpec("P-256");
        ECDomainParameters ecDomain = new ECDomainParameters(
                ecSpec.getCurve(),
                ecSpec.getG(),
                ecSpec.getN(),
                ecSpec.getH()
        );

        // Extract the EC point from the public key
        byte[] encodedPoint = publicKey.getEncoded();
        ECPoint point = ecSpec.getCurve().decodePoint(encodedPoint);

        ECPublicKeyParameters publicKeyParameters = new ECPublicKeyParameters(
                point, ecDomain
        );

        IESEngine iesEngine = new IESEngine(
                new org.bouncycastle.crypto.agreement.ECDHBasicAgreement(),
                new org.bouncycastle.crypto.generators.KDF2BytesGenerator(new SHA256Digest()),
                new HMac(new SHA256Digest())
        );

        iesEngine.init(true, publicKeyParameters, null, new IESWithCipherParameters(null, null, 128, 128));

        return iesEngine.processBlock(plaintext, 0, plaintext.length);
    }
}