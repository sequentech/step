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

import java.math.BigInteger;
import java.security.*;
import java.security.interfaces.ECPrivateKey;
import java.security.interfaces.ECPublicKey;
import java.security.spec.ECGenParameterSpec;
import java.util.Base64;

public class ECIESEncryptionExample {

    public static void main(String[] args) throws Exception {
        Security.addProvider(new BouncyCastleProvider());

        // Generate EC key pair (P-256 curve)
        KeyPairGenerator keyGen = KeyPairGenerator.getInstance("EC", "BC");
        keyGen.initialize(new ECGenParameterSpec("secp256r1"), new SecureRandom());
        KeyPair keyPair = keyGen.generateKeyPair();

        // Get the curve parameters
        ECParameterSpec ecSpec = ECNamedCurveTable.getParameterSpec("P-256");
        ECDomainParameters domainParams = new ECDomainParameters(
            ecSpec.getCurve(),
            ecSpec.getG(),
            ecSpec.getN(),
            ecSpec.getH()
        );

        // Convert to BouncyCastle compatible parameters
        ECPublicKey javaPublicKey = (ECPublicKey) keyPair.getPublic();
        ECPoint bcPublicPoint = ecSpec.getCurve().createPoint(
            new BigInteger(1, javaPublicKey.getW().getAffineX().toByteArray()),
            new BigInteger(1, javaPublicKey.getW().getAffineY().toByteArray())
        );
        ECPublicKeyParameters publicKeyParams = new ECPublicKeyParameters(bcPublicPoint, domainParams);

        ECPrivateKey javaPrivateKey = (ECPrivateKey) keyPair.getPrivate();
        BigInteger privateKeyValue = javaPrivateKey.getS();
        ECPrivateKeyParameters privateKeyParams = new ECPrivateKeyParameters(privateKeyValue, domainParams);

        // Set up IESEngine with ECDH, KDF2 (SHA-1), and AES-128-CBC with padding
        IESEngine iesEngine = new IESEngine(
                new ECDHBasicAgreement(),
                new KDF2BytesGenerator(new SHA1Digest()),
                new HMac(new SHA1Digest()),
                new PaddedBufferedBlockCipher(new CBCBlockCipher(new AESEngine()))
        );

        // Encryption parameters (128-bit AES key, 128-bit MAC key)
        IESWithCipherParameters params = new IESWithCipherParameters(null, null, 128, 128);

        // Initialize the engine for encryption (sender's private key and recipient's public key)
        iesEngine.init(true, privateKeyParams, publicKeyParams, params);

        // Plaintext to be encrypted
        byte[] plaintext = "Hello, ECIES!".getBytes();

        // Encrypt
        byte[] ciphertext = iesEngine.processBlock(plaintext, 0, plaintext.length);
        String encryptedBase64 = Base64.getEncoder().encodeToString(ciphertext);

        System.out.println("Encrypted text (Base64): " + encryptedBase64);

        // Initialize the engine for decryption (recipient's private key and sender's public key)
        iesEngine.init(false, privateKeyParams, publicKeyParams, params);
        byte[] decryptedText = iesEngine.processBlock(ciphertext, 0, ciphertext.length);

        System.out.println("Decrypted text: " + new String(decryptedText));
    }
}
