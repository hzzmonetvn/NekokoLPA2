import 'dart:convert';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:nlpa2/utils/aes_cipher.dart';
import 'package:nlpa2/utils/crypto_utils.dart';
import 'package:nlpa2/utils/hex_utils.dart';

void main() {
  group('AesCipher tests', () {
    test('AES-128 known answer vector', () {
      final key = HexUtils.hexToBytes('2b7e151628aed2a6abf7158809cf4f3c');
      final input = HexUtils.hexToBytes('3243f6a8885a308d313198a2e0370734');
      final cipher = AesCipher(key);

      final block = Uint8List.fromList(input);
      cipher.encryptBlock(block);
      expect(HexUtils.bytesToHex(block), '3925841d02dc09fbdc118597196a0b32');

      cipher.decryptBlock(block);
      expect(HexUtils.bytesToHex(block), '3243f6a8885a308d313198a2e0370734');
    });

    test('AES-256 known answer vector', () {
      final key = HexUtils.hexToBytes(
        '603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4',
      );
      final input = HexUtils.hexToBytes('6bc1bee22e409f96e93d7e117393172a');
      final cipher = AesCipher(key);

      final block = Uint8List.fromList(input);
      cipher.encryptBlock(block);
      expect(HexUtils.bytesToHex(block), 'f3eed1bdb5d2a03c064b5a7e3db181f8');

      cipher.decryptBlock(block);
      expect(HexUtils.bytesToHex(block), '6bc1bee22e409f96e93d7e117393172a');
    });

    test('AES CBC and PKCS7 padding roundtrip', () {
      final key = Uint8List(32)..fillRange(0, 32, 0x42);
      final iv = Uint8List(16)..fillRange(0, 16, 0x24);
      final plaintext = utf8.encode('NekokoLPA2 pure Dart zero-dependency AES cipher');

      final padded = AesCipher.pkcs7Pad(Uint8List.fromList(plaintext));
      expect(padded.length % 16, 0);

      final encrypted = AesCipher.cbcEncrypt(key, iv, padded);
      final decryptedPadded = AesCipher.cbcDecrypt(key, iv, encrypted);
      final unpadded = AesCipher.pkcs7Unpad(decryptedPadded);

      expect(unpadded, plaintext);
    });

    test('CryptoUtils encryptAES and decryptAES roundtrip', () {
      const password = 'StrongPassword123!';
      const originalText = '{"action":"authenticate","data":"01020304"}';

      final ciphertext = CryptoUtils.encryptAES(originalText, password);
      final decrypted = CryptoUtils.decryptAES(ciphertext, password);

      expect(decrypted, originalText);
    });
  });
}
