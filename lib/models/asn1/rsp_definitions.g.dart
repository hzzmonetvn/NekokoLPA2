import 'dart:typed_data';
import 'dart:convert';
import './simple_reader.dart';

// GENERATED FILE - do not edit.
// Source: tools/asn1/*.asn1  Regenerate: python3 tools/asn1_dart_gen.py

Uint8List _asn1EncodeLength(int length) {
  if (length < 0x80) return Uint8List.fromList([length]);
  final bytes = <int>[];
  var v = length;
  while (v > 0) {
    bytes.insert(0, v & 0xFF);
    v >>= 8;
  }
  bytes.insert(0, 0x80 | bytes.length);
  return Uint8List.fromList(bytes);
}

Uint8List _asn1EncodeInt(int value) {
  if (value == 0) return Uint8List.fromList([0]);
  List<int> bytes = [];
  int v = value;
  for (int i = 0; i < 8; i++) {
    bytes.insert(0, v & 0xFF);
    v >>= 8;
  }
  while (bytes.length > 1) {
    if (bytes[0] == 0x00 && (bytes[1] & 0x80) == 0) {
      bytes.removeAt(0);
    } else if (bytes[0] == 0xFF && (bytes[1] & 0x80) != 0) {
      bytes.removeAt(0);
    } else {
      break;
    }
  }
  return Uint8List.fromList(bytes);
}

int _asn1DecodeInt(Uint8List bytes) {
  if (bytes.isEmpty) return 0;
  int v = bytes[0] >= 0x80 ? -1 : 0;
  for (final b in bytes) {
    v = (v << 8) | b;
  }
  return v;
}

Uint8List _asn1EncodeBits(int value) {
  if (value == 0) return Uint8List.fromList([0, 0]);
  int maxBit = -1;
  for (int i = 63; i >= 0; i--) {
    if ((value & (1 << i)) != 0) {
      maxBit = i;
      break;
    }
  }
  if (maxBit == -1) return Uint8List.fromList([0, 0]);
  int numDataBytes = (maxBit / 8).floor() + 1;
  int unused = (8 - (maxBit + 1) % 8) % 8;
  final out = Uint8List(1 + numDataBytes);
  out[0] = unused;
  for (int i = 0; i <= maxBit; i++) {
    if ((value & (1 << i)) != 0) {
      int byteIdx = (i / 8).floor() + 1;
      int bitInByte = 7 - (i % 8);
      out[byteIdx] |= (1 << bitInByte);
    }
  }
  return out;
}

int _asn1DecodeBits(Uint8List bytes) {
  if (bytes.length < 2) return 0;
  int unused = bytes[0];
  int result = 0;
  for (int i = 1; i < bytes.length; i++) {
    int b = bytes[i];
    if (i == bytes.length - 1) {
      b &= (0xFF << unused) & 0xFF;
    }
    for (int j = 0; j < 8; j++) {
      if ((b & (0x80 >> j)) != 0) {
        int bitIdx = (i - 1) * 8 + j;
        if (bitIdx < 64) {
          result |= (1 << bitIdx);
        }
      }
    }
  }
  return result;
}

List<int> _tagBytes(int tag) {
  final out = <int>[];
  var v = tag;
  while (v > 0) {
    out.insert(0, v & 0xFF);
    v >>= 8;
  }
  return out;
}

Uint8List _asn1EncodeRaw(int tag, Uint8List value) {
  final tb = _tagBytes(tag);
  final lb = _asn1EncodeLength(value.length);
  final out = Uint8List(tb.length + lb.length + value.length);
  out.setRange(0, tb.length, tb);
  out.setRange(tb.length, tb.length + lb.length, lb);
  out.setRange(tb.length + lb.length, out.length, value);
  return out;
}

/// Concatenates already-encoded children under [tag].
///
/// Sized once and filled, rather than folded: growing a new list per
/// child copies everything before it, which made encoding a long
/// SEQUENCE OF quadratic in its length.
Uint8List _asn1EncodeConstructed(int tag, List<Uint8List> children) {
  var total = 0;
  for (final child in children) {
    total += child.length;
  }
  final body = Uint8List(total);
  var offset = 0;
  for (final child in children) {
    body.setRange(offset, offset + child.length, child);
    offset += child.length;
  }
  return _asn1EncodeRaw(tag, body);
}

/// Clears the constructed bit in a tag's leading identifier octet.
int _tagClass(int tag) {
  var shift = 0;
  var head = tag;
  while (head > 0xFF) {
    head >>= 8;
    shift += 8;
  }
  return tag & ~(0x20 << shift);
}

/// Index of the first child at or after [from] carrying [tag], else -1.
///
/// SEQUENCE members are located by tag rather than by position: these
/// modules are EXTENSIBILITY IMPLIED, so a newer card may send elements a
/// build does not know about, and OPTIONAL members may simply be absent.
/// Counting positions instead makes every following member decode into
/// the wrong field.
///
/// The constructed bit is ignored when matching. X.680 requires the
/// members of a SEQUENCE to have distinct tag numbers, so this cannot be
/// ambiguous, and cards do disagree with the schema about it: EUICCInfo2
/// carries extCardResource as a primitive '84' even though its content is
/// a constructed structure.
int _seekTag(List<Asn1Node> children, int from, int tag) {
  final want = _tagClass(tag);
  for (var i = from; i < children.length; i++) {
    if (_tagClass(children[i].tag) == want) return i;
  }
  return -1;
}

const String _hexDigits = '0123456789ABCDEF';

/// Uppercase hex for toString(). A nibble at a time into one
/// buffer, rather than a String per byte.
String _hex(dynamic bytes) {
  if (bytes is! Uint8List) return bytes.toString();
  final out = Uint8List(bytes.length * 2);
  var i = 0;
  for (final b in bytes) {
    out[i++] = _hexDigits.codeUnitAt((b >> 4) & 0x0F);
    out[i++] = _hexDigits.codeUnitAt(b & 0x0F);
  }
  return String.fromCharCodes(out);
}

enum Version {
  v1, // 0
  v2, // 1
  v3, // 2
}

extension VersionExt on Version {
  int get value {
    switch (this) {
      case Version.v1:
        return 0;
      case Version.v2:
        return 1;
      case Version.v3:
        return 2;
    }
  }

  static Version? fromValue(int v) {
    switch (v) {
      case 0:
        return Version.v1;
      case 1:
        return Version.v2;
      case 2:
        return Version.v3;
      default:
        return null;
    }
  }
}

enum TerminalType {
  telex, // 3
  teletex, // 4
  g3_facsimile, // 5
  g4_facsimile, // 6
  ia5_terminal, // 7
  videotex, // 8
}

extension TerminalTypeExt on TerminalType {
  int get value {
    switch (this) {
      case TerminalType.telex:
        return 3;
      case TerminalType.teletex:
        return 4;
      case TerminalType.g3_facsimile:
        return 5;
      case TerminalType.g4_facsimile:
        return 6;
      case TerminalType.ia5_terminal:
        return 7;
      case TerminalType.videotex:
        return 8;
    }
  }

  static TerminalType? fromValue(int v) {
    switch (v) {
      case 3:
        return TerminalType.telex;
      case 4:
        return TerminalType.teletex;
      case 5:
        return TerminalType.g3_facsimile;
      case 6:
        return TerminalType.g4_facsimile;
      case 7:
        return TerminalType.ia5_terminal;
      case 8:
        return TerminalType.videotex;
      default:
        return null;
    }
  }
}

class UICCCapability {
  final int value;
  const UICCCapability(this.value);

  static const int contactlessSupport = 1 << 0;
  static const int usimSupport = 1 << 1;
  static const int isimSupport = 1 << 2;
  static const int csimSupport = 1 << 3;
  static const int akaMilenage = 1 << 4;
  static const int akaCave = 1 << 5;
  static const int akaTuak128 = 1 << 6;
  static const int akaTuak256 = 1 << 7;
  static const int usimTestAlgorithm = 1 << 8;
  static const int rfu2 = 1 << 9;
  static const int gbaAuthenUsim = 1 << 10;
  static const int gbaAuthenISim = 1 << 11;
  static const int mbmsAuthenUsim = 1 << 12;
  static const int eapClient = 1 << 13;
  static const int javacard = 1 << 14;
  static const int multos = 1 << 15;
  static const int multipleUsimSupport = 1 << 16;
  static const int multipleIsimSupport = 1 << 17;
  static const int multipleCsimSupport = 1 << 18;
  static const int berTlvFileSupport = 1 << 19;
  static const int dfLinkSupport = 1 << 20;
  static const int catTp = 1 << 21;
  static const int getIdentity = 1 << 22;
  static const int profile_a_x25519 = 1 << 23;
  static const int profile_b_p256 = 1 << 24;
  static const int suciCalculatorApi = 1 << 25;
  static const int dns_resolution = 1 << 26;
  static const int scp11ac = 1 << 27;
  static const int scp11c_authorization_mechanism = 1 << 28;
  static const int s16mode = 1 << 29;
  static const int eaka = 1 << 30;
  static const int iotminimal = 1 << 31;
  static const int suci_nswo = 1 << 32;
  static const int network_id = 1 << 33;
  static const int coap = 1 << 34;
  static const int gbauApi = 1 << 35;
  static const int uir5GProSe = 1 << 36;
  static const int ssim_Eap_Tls = 1 << 37;
  static const int ssim_Eap_Aka_prime = 1 << 38;
  static const int usatAppPairing = 1 << 39;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & contactlessSupport) != 0)
      parts.add('UICCCapability.contactlessSupport');
    if ((value & usimSupport) != 0) parts.add('UICCCapability.usimSupport');
    if ((value & isimSupport) != 0) parts.add('UICCCapability.isimSupport');
    if ((value & csimSupport) != 0) parts.add('UICCCapability.csimSupport');
    if ((value & akaMilenage) != 0) parts.add('UICCCapability.akaMilenage');
    if ((value & akaCave) != 0) parts.add('UICCCapability.akaCave');
    if ((value & akaTuak128) != 0) parts.add('UICCCapability.akaTuak128');
    if ((value & akaTuak256) != 0) parts.add('UICCCapability.akaTuak256');
    if ((value & usimTestAlgorithm) != 0)
      parts.add('UICCCapability.usimTestAlgorithm');
    if ((value & rfu2) != 0) parts.add('UICCCapability.rfu2');
    if ((value & gbaAuthenUsim) != 0) parts.add('UICCCapability.gbaAuthenUsim');
    if ((value & gbaAuthenISim) != 0) parts.add('UICCCapability.gbaAuthenISim');
    if ((value & mbmsAuthenUsim) != 0)
      parts.add('UICCCapability.mbmsAuthenUsim');
    if ((value & eapClient) != 0) parts.add('UICCCapability.eapClient');
    if ((value & javacard) != 0) parts.add('UICCCapability.javacard');
    if ((value & multos) != 0) parts.add('UICCCapability.multos');
    if ((value & multipleUsimSupport) != 0)
      parts.add('UICCCapability.multipleUsimSupport');
    if ((value & multipleIsimSupport) != 0)
      parts.add('UICCCapability.multipleIsimSupport');
    if ((value & multipleCsimSupport) != 0)
      parts.add('UICCCapability.multipleCsimSupport');
    if ((value & berTlvFileSupport) != 0)
      parts.add('UICCCapability.berTlvFileSupport');
    if ((value & dfLinkSupport) != 0) parts.add('UICCCapability.dfLinkSupport');
    if ((value & catTp) != 0) parts.add('UICCCapability.catTp');
    if ((value & getIdentity) != 0) parts.add('UICCCapability.getIdentity');
    if ((value & profile_a_x25519) != 0)
      parts.add('UICCCapability.profile_a_x25519');
    if ((value & profile_b_p256) != 0)
      parts.add('UICCCapability.profile_b_p256');
    if ((value & suciCalculatorApi) != 0)
      parts.add('UICCCapability.suciCalculatorApi');
    if ((value & dns_resolution) != 0)
      parts.add('UICCCapability.dns_resolution');
    if ((value & scp11ac) != 0) parts.add('UICCCapability.scp11ac');
    if ((value & scp11c_authorization_mechanism) != 0)
      parts.add('UICCCapability.scp11c_authorization_mechanism');
    if ((value & s16mode) != 0) parts.add('UICCCapability.s16mode');
    if ((value & eaka) != 0) parts.add('UICCCapability.eaka');
    if ((value & iotminimal) != 0) parts.add('UICCCapability.iotminimal');
    if ((value & suci_nswo) != 0) parts.add('UICCCapability.suci_nswo');
    if ((value & network_id) != 0) parts.add('UICCCapability.network_id');
    if ((value & coap) != 0) parts.add('UICCCapability.coap');
    if ((value & gbauApi) != 0) parts.add('UICCCapability.gbauApi');
    if ((value & uir5GProSe) != 0) parts.add('UICCCapability.uir5GProSe');
    if ((value & ssim_Eap_Tls) != 0) parts.add('UICCCapability.ssim_Eap_Tls');
    if ((value & ssim_Eap_Aka_prime) != 0)
      parts.add('UICCCapability.ssim_Eap_Aka_prime');
    if ((value & usatAppPairing) != 0)
      parts.add('UICCCapability.usatAppPairing');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

enum AlgoParameter_algorithmID {
  milenage, // 1
  tuak, // 2
  usim_test_algorithm, // 3
}

extension AlgoParameter_algorithmIDExt on AlgoParameter_algorithmID {
  int get value {
    switch (this) {
      case AlgoParameter_algorithmID.milenage:
        return 1;
      case AlgoParameter_algorithmID.tuak:
        return 2;
      case AlgoParameter_algorithmID.usim_test_algorithm:
        return 3;
    }
  }

  static AlgoParameter_algorithmID? fromValue(int v) {
    switch (v) {
      case 1:
        return AlgoParameter_algorithmID.milenage;
      case 2:
        return AlgoParameter_algorithmID.tuak;
      case 3:
        return AlgoParameter_algorithmID.usim_test_algorithm;
      default:
        return null;
    }
  }
}

enum PINKeyReferenceValue {
  pinAppl1, // 1
  pinAppl2, // 2
  pinAppl3, // 3
  pinAppl4, // 4
  pinAppl5, // 5
  pinAppl6, // 6
  pinAppl7, // 7
  pinAppl8, // 8
  adm1, // 10
  adm2, // 11
  adm3, // 12
  adm4, // 13
  adm5, // 14
  secondPINAppl1, // 129
  secondPINAppl2, // 130
  secondPINAppl3, // 131
  secondPINAppl4, // 132
  secondPINAppl5, // 133
  secondPINAppl6, // 134
  secondPINAppl7, // 135
  secondPINAppl8, // 136
  adm6, // 138
  adm7, // 139
  adm8, // 140
  adm9, // 141
  adm10, // 142
}

extension PINKeyReferenceValueExt on PINKeyReferenceValue {
  int get value {
    switch (this) {
      case PINKeyReferenceValue.pinAppl1:
        return 1;
      case PINKeyReferenceValue.pinAppl2:
        return 2;
      case PINKeyReferenceValue.pinAppl3:
        return 3;
      case PINKeyReferenceValue.pinAppl4:
        return 4;
      case PINKeyReferenceValue.pinAppl5:
        return 5;
      case PINKeyReferenceValue.pinAppl6:
        return 6;
      case PINKeyReferenceValue.pinAppl7:
        return 7;
      case PINKeyReferenceValue.pinAppl8:
        return 8;
      case PINKeyReferenceValue.adm1:
        return 10;
      case PINKeyReferenceValue.adm2:
        return 11;
      case PINKeyReferenceValue.adm3:
        return 12;
      case PINKeyReferenceValue.adm4:
        return 13;
      case PINKeyReferenceValue.adm5:
        return 14;
      case PINKeyReferenceValue.secondPINAppl1:
        return 129;
      case PINKeyReferenceValue.secondPINAppl2:
        return 130;
      case PINKeyReferenceValue.secondPINAppl3:
        return 131;
      case PINKeyReferenceValue.secondPINAppl4:
        return 132;
      case PINKeyReferenceValue.secondPINAppl5:
        return 133;
      case PINKeyReferenceValue.secondPINAppl6:
        return 134;
      case PINKeyReferenceValue.secondPINAppl7:
        return 135;
      case PINKeyReferenceValue.secondPINAppl8:
        return 136;
      case PINKeyReferenceValue.adm6:
        return 138;
      case PINKeyReferenceValue.adm7:
        return 139;
      case PINKeyReferenceValue.adm8:
        return 140;
      case PINKeyReferenceValue.adm9:
        return 141;
      case PINKeyReferenceValue.adm10:
        return 142;
    }
  }

  static PINKeyReferenceValue? fromValue(int v) {
    switch (v) {
      case 1:
        return PINKeyReferenceValue.pinAppl1;
      case 2:
        return PINKeyReferenceValue.pinAppl2;
      case 3:
        return PINKeyReferenceValue.pinAppl3;
      case 4:
        return PINKeyReferenceValue.pinAppl4;
      case 5:
        return PINKeyReferenceValue.pinAppl5;
      case 6:
        return PINKeyReferenceValue.pinAppl6;
      case 7:
        return PINKeyReferenceValue.pinAppl7;
      case 8:
        return PINKeyReferenceValue.pinAppl8;
      case 10:
        return PINKeyReferenceValue.adm1;
      case 11:
        return PINKeyReferenceValue.adm2;
      case 12:
        return PINKeyReferenceValue.adm3;
      case 13:
        return PINKeyReferenceValue.adm4;
      case 14:
        return PINKeyReferenceValue.adm5;
      case 129:
        return PINKeyReferenceValue.secondPINAppl1;
      case 130:
        return PINKeyReferenceValue.secondPINAppl2;
      case 131:
        return PINKeyReferenceValue.secondPINAppl3;
      case 132:
        return PINKeyReferenceValue.secondPINAppl4;
      case 133:
        return PINKeyReferenceValue.secondPINAppl5;
      case 134:
        return PINKeyReferenceValue.secondPINAppl6;
      case 135:
        return PINKeyReferenceValue.secondPINAppl7;
      case 136:
        return PINKeyReferenceValue.secondPINAppl8;
      case 138:
        return PINKeyReferenceValue.adm6;
      case 139:
        return PINKeyReferenceValue.adm7;
      case 140:
        return PINKeyReferenceValue.adm8;
      case 141:
        return PINKeyReferenceValue.adm9;
      case 142:
        return PINKeyReferenceValue.adm10;
      default:
        return null;
    }
  }
}

enum PUKKeyReferenceValue {
  pukAppl1, // 1
  pukAppl2, // 2
  pukAppl3, // 3
  pukAppl4, // 4
  pukAppl5, // 5
  pukAppl6, // 6
  pukAppl7, // 7
  pukAppl8, // 8
  secondPUKAppl1, // 129
  secondPUKAppl2, // 130
  secondPUKAppl3, // 131
  secondPUKAppl4, // 132
  secondPUKAppl5, // 133
  secondPUKAppl6, // 134
  secondPUKAppl7, // 135
  secondPUKAppl8, // 136
}

extension PUKKeyReferenceValueExt on PUKKeyReferenceValue {
  int get value {
    switch (this) {
      case PUKKeyReferenceValue.pukAppl1:
        return 1;
      case PUKKeyReferenceValue.pukAppl2:
        return 2;
      case PUKKeyReferenceValue.pukAppl3:
        return 3;
      case PUKKeyReferenceValue.pukAppl4:
        return 4;
      case PUKKeyReferenceValue.pukAppl5:
        return 5;
      case PUKKeyReferenceValue.pukAppl6:
        return 6;
      case PUKKeyReferenceValue.pukAppl7:
        return 7;
      case PUKKeyReferenceValue.pukAppl8:
        return 8;
      case PUKKeyReferenceValue.secondPUKAppl1:
        return 129;
      case PUKKeyReferenceValue.secondPUKAppl2:
        return 130;
      case PUKKeyReferenceValue.secondPUKAppl3:
        return 131;
      case PUKKeyReferenceValue.secondPUKAppl4:
        return 132;
      case PUKKeyReferenceValue.secondPUKAppl5:
        return 133;
      case PUKKeyReferenceValue.secondPUKAppl6:
        return 134;
      case PUKKeyReferenceValue.secondPUKAppl7:
        return 135;
      case PUKKeyReferenceValue.secondPUKAppl8:
        return 136;
    }
  }

  static PUKKeyReferenceValue? fromValue(int v) {
    switch (v) {
      case 1:
        return PUKKeyReferenceValue.pukAppl1;
      case 2:
        return PUKKeyReferenceValue.pukAppl2;
      case 3:
        return PUKKeyReferenceValue.pukAppl3;
      case 4:
        return PUKKeyReferenceValue.pukAppl4;
      case 5:
        return PUKKeyReferenceValue.pukAppl5;
      case 6:
        return PUKKeyReferenceValue.pukAppl6;
      case 7:
        return PUKKeyReferenceValue.pukAppl7;
      case 8:
        return PUKKeyReferenceValue.pukAppl8;
      case 129:
        return PUKKeyReferenceValue.secondPUKAppl1;
      case 130:
        return PUKKeyReferenceValue.secondPUKAppl2;
      case 131:
        return PUKKeyReferenceValue.secondPUKAppl3;
      case 132:
        return PUKKeyReferenceValue.secondPUKAppl4;
      case 133:
        return PUKKeyReferenceValue.secondPUKAppl5;
      case 134:
        return PUKKeyReferenceValue.secondPUKAppl6;
      case 135:
        return PUKKeyReferenceValue.secondPUKAppl7;
      case 136:
        return PUKKeyReferenceValue.secondPUKAppl8;
      default:
        return null;
    }
  }
}

enum PEStatus_status {
  ok, // 0
  pe_not_supported, // 1
  memory_failure, // 2
  bad_values, // 3
  not_enough_memory, // 4
  invalid_request_format, // 5
  invalid_parameter, // 6
  runtime_not_supported, // 7
  lib_not_supported, // 8
  template_not_supported, // 9
  feature_not_supported, // 10
  pin_code_missing, // 11
  unsupported_profile_version, // 31
}

extension PEStatus_statusExt on PEStatus_status {
  int get value {
    switch (this) {
      case PEStatus_status.ok:
        return 0;
      case PEStatus_status.pe_not_supported:
        return 1;
      case PEStatus_status.memory_failure:
        return 2;
      case PEStatus_status.bad_values:
        return 3;
      case PEStatus_status.not_enough_memory:
        return 4;
      case PEStatus_status.invalid_request_format:
        return 5;
      case PEStatus_status.invalid_parameter:
        return 6;
      case PEStatus_status.runtime_not_supported:
        return 7;
      case PEStatus_status.lib_not_supported:
        return 8;
      case PEStatus_status.template_not_supported:
        return 9;
      case PEStatus_status.feature_not_supported:
        return 10;
      case PEStatus_status.pin_code_missing:
        return 11;
      case PEStatus_status.unsupported_profile_version:
        return 31;
    }
  }

  static PEStatus_status? fromValue(int v) {
    switch (v) {
      case 0:
        return PEStatus_status.ok;
      case 1:
        return PEStatus_status.pe_not_supported;
      case 2:
        return PEStatus_status.memory_failure;
      case 3:
        return PEStatus_status.bad_values;
      case 4:
        return PEStatus_status.not_enough_memory;
      case 5:
        return PEStatus_status.invalid_request_format;
      case 6:
        return PEStatus_status.invalid_parameter;
      case 7:
        return PEStatus_status.runtime_not_supported;
      case 8:
        return PEStatus_status.lib_not_supported;
      case 9:
        return PEStatus_status.template_not_supported;
      case 10:
        return PEStatus_status.feature_not_supported;
      case 11:
        return PEStatus_status.pin_code_missing;
      case 31:
        return PEStatus_status.unsupported_profile_version;
      default:
        return null;
    }
  }
}

enum RemoteOpId {
  installBoundProfilePackage, // 1
}

extension RemoteOpIdExt on RemoteOpId {
  int get value {
    switch (this) {
      case RemoteOpId.installBoundProfilePackage:
        return 1;
    }
  }

  static RemoteOpId? fromValue(int v) {
    switch (v) {
      case 1:
        return RemoteOpId.installBoundProfilePackage;
      default:
        return null;
    }
  }
}

class PprIds {
  final int value;
  const PprIds(this.value);

  static const int pprUpdateControl = 1 << 0;
  static const int ppr1 = 1 << 1;
  static const int ppr2 = 1 << 2;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & pprUpdateControl) != 0) parts.add('PprIds.pprUpdateControl');
    if ((value & ppr1) != 0) parts.add('PprIds.ppr1');
    if ((value & ppr2) != 0) parts.add('PprIds.ppr2');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

class RpmType {
  final int value;
  const RpmType(this.value);

  static const int enable = 1 << 0;
  static const int disable = 1 << 1;
  static const int delete = 1 << 2;
  static const int listProfileInfo = 1 << 3;
  static const int contactPcmp = 1 << 4;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & enable) != 0) parts.add('RpmType.enable');
    if ((value & disable) != 0) parts.add('RpmType.disable');
    if ((value & delete) != 0) parts.add('RpmType.delete');
    if ((value & listProfileInfo) != 0) parts.add('RpmType.listProfileInfo');
    if ((value & contactPcmp) != 0) parts.add('RpmType.contactPcmp');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

class CatSupportedClasses {
  final int value;
  const CatSupportedClasses(this.value);

  static const int a = 1 << 0;
  static const int b = 1 << 1;
  static const int c = 1 << 2;
  static const int d = 1 << 3;
  static const int e = 1 << 4;
  static const int f = 1 << 5;
  static const int g = 1 << 6;
  static const int h = 1 << 7;
  static const int i = 1 << 8;
  static const int j = 1 << 9;
  static const int k = 1 << 10;
  static const int l = 1 << 11;
  static const int m = 1 << 12;
  static const int n = 1 << 13;
  static const int o = 1 << 14;
  static const int p = 1 << 15;
  static const int q = 1 << 16;
  static const int r = 1 << 17;
  static const int s = 1 << 18;
  static const int t = 1 << 19;
  static const int u = 1 << 20;
  static const int v = 1 << 21;
  static const int w = 1 << 22;
  static const int x = 1 << 23;
  static const int y = 1 << 24;
  static const int z = 1 << 25;
  static const int aa = 1 << 26;
  static const int ab = 1 << 27;
  static const int ac = 1 << 28;
  static const int ad = 1 << 29;
  static const int ae = 1 << 30;
  static const int af = 1 << 31;
  static const int ag = 1 << 32;
  static const int ah = 1 << 33;
  static const int ai = 1 << 34;
  static const int aj = 1 << 35;
  static const int ak = 1 << 36;
  static const int al = 1 << 37;
  static const int am = 1 << 38;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & a) != 0) parts.add('CatSupportedClasses.a');
    if ((value & b) != 0) parts.add('CatSupportedClasses.b');
    if ((value & c) != 0) parts.add('CatSupportedClasses.c');
    if ((value & d) != 0) parts.add('CatSupportedClasses.d');
    if ((value & e) != 0) parts.add('CatSupportedClasses.e');
    if ((value & f) != 0) parts.add('CatSupportedClasses.f');
    if ((value & g) != 0) parts.add('CatSupportedClasses.g');
    if ((value & h) != 0) parts.add('CatSupportedClasses.h');
    if ((value & i) != 0) parts.add('CatSupportedClasses.i');
    if ((value & j) != 0) parts.add('CatSupportedClasses.j');
    if ((value & k) != 0) parts.add('CatSupportedClasses.k');
    if ((value & l) != 0) parts.add('CatSupportedClasses.l');
    if ((value & m) != 0) parts.add('CatSupportedClasses.m');
    if ((value & n) != 0) parts.add('CatSupportedClasses.n');
    if ((value & o) != 0) parts.add('CatSupportedClasses.o');
    if ((value & p) != 0) parts.add('CatSupportedClasses.p');
    if ((value & q) != 0) parts.add('CatSupportedClasses.q');
    if ((value & r) != 0) parts.add('CatSupportedClasses.r');
    if ((value & s) != 0) parts.add('CatSupportedClasses.s');
    if ((value & t) != 0) parts.add('CatSupportedClasses.t');
    if ((value & u) != 0) parts.add('CatSupportedClasses.u');
    if ((value & v) != 0) parts.add('CatSupportedClasses.v');
    if ((value & w) != 0) parts.add('CatSupportedClasses.w');
    if ((value & x) != 0) parts.add('CatSupportedClasses.x');
    if ((value & y) != 0) parts.add('CatSupportedClasses.y');
    if ((value & z) != 0) parts.add('CatSupportedClasses.z');
    if ((value & aa) != 0) parts.add('CatSupportedClasses.aa');
    if ((value & ab) != 0) parts.add('CatSupportedClasses.ab');
    if ((value & ac) != 0) parts.add('CatSupportedClasses.ac');
    if ((value & ad) != 0) parts.add('CatSupportedClasses.ad');
    if ((value & ae) != 0) parts.add('CatSupportedClasses.ae');
    if ((value & af) != 0) parts.add('CatSupportedClasses.af');
    if ((value & ag) != 0) parts.add('CatSupportedClasses.ag');
    if ((value & ah) != 0) parts.add('CatSupportedClasses.ah');
    if ((value & ai) != 0) parts.add('CatSupportedClasses.ai');
    if ((value & aj) != 0) parts.add('CatSupportedClasses.aj');
    if ((value & ak) != 0) parts.add('CatSupportedClasses.ak');
    if ((value & al) != 0) parts.add('CatSupportedClasses.al');
    if ((value & am) != 0) parts.add('CatSupportedClasses.am');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

enum EuiccFormFactorType {
  removableEuicc, // 0
  nonRemovableEuicc, // 1
}

extension EuiccFormFactorTypeExt on EuiccFormFactorType {
  int get value {
    switch (this) {
      case EuiccFormFactorType.removableEuicc:
        return 0;
      case EuiccFormFactorType.nonRemovableEuicc:
        return 1;
    }
  }

  static EuiccFormFactorType? fromValue(int v) {
    switch (v) {
      case 0:
        return EuiccFormFactorType.removableEuicc;
      case 1:
        return EuiccFormFactorType.nonRemovableEuicc;
      default:
        return null;
    }
  }
}

class LpaRspCapability {
  final int value;
  const LpaRspCapability(this.value);

  static const int crlStaplingV3Support = 1 << 0;
  static const int certChainV3Support = 1 << 1;
  static const int apduApiSupport = 1 << 2;
  static const int enterpriseCapableDevice = 1 << 3;
  static const int lpaProxySupport = 1 << 4;
  static const int signedSmdsResponseV3Support = 1 << 5;
  static const int euiccCiUpdateSupport = 1 << 6;
  static const int eventCheckingSupport = 1 << 7;
  static const int pushServiceSupport = 1 << 8;
  static const int pendingOperationAlertingSupport = 1 << 9;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & crlStaplingV3Support) != 0)
      parts.add('LpaRspCapability.crlStaplingV3Support');
    if ((value & certChainV3Support) != 0)
      parts.add('LpaRspCapability.certChainV3Support');
    if ((value & apduApiSupport) != 0)
      parts.add('LpaRspCapability.apduApiSupport');
    if ((value & enterpriseCapableDevice) != 0)
      parts.add('LpaRspCapability.enterpriseCapableDevice');
    if ((value & lpaProxySupport) != 0)
      parts.add('LpaRspCapability.lpaProxySupport');
    if ((value & signedSmdsResponseV3Support) != 0)
      parts.add('LpaRspCapability.signedSmdsResponseV3Support');
    if ((value & euiccCiUpdateSupport) != 0)
      parts.add('LpaRspCapability.euiccCiUpdateSupport');
    if ((value & eventCheckingSupport) != 0)
      parts.add('LpaRspCapability.eventCheckingSupport');
    if ((value & pushServiceSupport) != 0)
      parts.add('LpaRspCapability.pushServiceSupport');
    if ((value & pendingOperationAlertingSupport) != 0)
      parts.add('LpaRspCapability.pendingOperationAlertingSupport');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

enum EUICCInfo2_euiccCategory {
  other, // 0
  basicEuicc, // 1
  mediumEuicc, // 2
  contactlessEuicc, // 3
}

extension EUICCInfo2_euiccCategoryExt on EUICCInfo2_euiccCategory {
  int get value {
    switch (this) {
      case EUICCInfo2_euiccCategory.other:
        return 0;
      case EUICCInfo2_euiccCategory.basicEuicc:
        return 1;
      case EUICCInfo2_euiccCategory.mediumEuicc:
        return 2;
      case EUICCInfo2_euiccCategory.contactlessEuicc:
        return 3;
    }
  }

  static EUICCInfo2_euiccCategory? fromValue(int v) {
    switch (v) {
      case 0:
        return EUICCInfo2_euiccCategory.other;
      case 1:
        return EUICCInfo2_euiccCategory.basicEuicc;
      case 2:
        return EUICCInfo2_euiccCategory.mediumEuicc;
      case 3:
        return EUICCInfo2_euiccCategory.contactlessEuicc;
      default:
        return null;
    }
  }
}

class EUICCInfo2_treProperties {
  final int value;
  const EUICCInfo2_treProperties(this.value);

  static const int isDiscrete = 1 << 0;
  static const int isIntegrated = 1 << 1;
  static const int usesRemoteMemory = 1 << 2;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & isDiscrete) != 0)
      parts.add('EUICCInfo2_treProperties.isDiscrete');
    if ((value & isIntegrated) != 0)
      parts.add('EUICCInfo2_treProperties.isIntegrated');
    if ((value & usesRemoteMemory) != 0)
      parts.add('EUICCInfo2_treProperties.usesRemoteMemory');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

class EuiccRspCapability {
  final int value;
  const EuiccRspCapability(this.value);

  static const int additionalProfile = 1 << 0;
  static const int loadCrlSupport = 1 << 1;
  static const int rpmSupport = 1 << 2;
  static const int testProfileSupport = 1 << 3;
  static const int deviceInfoExtensibilitySupport = 1 << 4;
  static const int serviceSpecificDataSupport = 1 << 5;
  static const int hriServerAddressSupport = 1 << 6;
  static const int serviceProviderMessageSupport = 1 << 7;
  static const int lpaProxySupport = 1 << 8;
  static const int enterpriseProfilesSupport = 1 << 9;
  static const int serviceDescriptionSupport = 1 << 10;
  static const int deviceChangeSupport = 1 << 11;
  static const int encryptedDeviceChangeDataSupport = 1 << 12;
  static const int estimatedProfileSizeIndicationSupport = 1 << 13;
  static const int profileSizeInProfilesInfoSupport = 1 << 14;
  static const int crlStaplingV3Support = 1 << 15;
  static const int certChainV3VerificationSupport = 1 << 16;
  static const int signedSmdsResponseV3Support = 1 << 17;
  static const int euiccRspCapInInfo1 = 1 << 18;
  static const int osUpdateSupport = 1 << 19;
  static const int cancelForEmptySpnPnSupport = 1 << 20;
  static const int updateNotifConfigInfoSupport = 1 << 21;
  static const int updateMetadataV3Support = 1 << 22;
  static const int v3ObjectsInCtxParamsCASupport = 1 << 23;
  static const int pushServiceRegistrationSupport = 1 << 24;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & additionalProfile) != 0)
      parts.add('EuiccRspCapability.additionalProfile');
    if ((value & loadCrlSupport) != 0)
      parts.add('EuiccRspCapability.loadCrlSupport');
    if ((value & rpmSupport) != 0) parts.add('EuiccRspCapability.rpmSupport');
    if ((value & testProfileSupport) != 0)
      parts.add('EuiccRspCapability.testProfileSupport');
    if ((value & deviceInfoExtensibilitySupport) != 0)
      parts.add('EuiccRspCapability.deviceInfoExtensibilitySupport');
    if ((value & serviceSpecificDataSupport) != 0)
      parts.add('EuiccRspCapability.serviceSpecificDataSupport');
    if ((value & hriServerAddressSupport) != 0)
      parts.add('EuiccRspCapability.hriServerAddressSupport');
    if ((value & serviceProviderMessageSupport) != 0)
      parts.add('EuiccRspCapability.serviceProviderMessageSupport');
    if ((value & lpaProxySupport) != 0)
      parts.add('EuiccRspCapability.lpaProxySupport');
    if ((value & enterpriseProfilesSupport) != 0)
      parts.add('EuiccRspCapability.enterpriseProfilesSupport');
    if ((value & serviceDescriptionSupport) != 0)
      parts.add('EuiccRspCapability.serviceDescriptionSupport');
    if ((value & deviceChangeSupport) != 0)
      parts.add('EuiccRspCapability.deviceChangeSupport');
    if ((value & encryptedDeviceChangeDataSupport) != 0)
      parts.add('EuiccRspCapability.encryptedDeviceChangeDataSupport');
    if ((value & estimatedProfileSizeIndicationSupport) != 0)
      parts.add('EuiccRspCapability.estimatedProfileSizeIndicationSupport');
    if ((value & profileSizeInProfilesInfoSupport) != 0)
      parts.add('EuiccRspCapability.profileSizeInProfilesInfoSupport');
    if ((value & crlStaplingV3Support) != 0)
      parts.add('EuiccRspCapability.crlStaplingV3Support');
    if ((value & certChainV3VerificationSupport) != 0)
      parts.add('EuiccRspCapability.certChainV3VerificationSupport');
    if ((value & signedSmdsResponseV3Support) != 0)
      parts.add('EuiccRspCapability.signedSmdsResponseV3Support');
    if ((value & euiccRspCapInInfo1) != 0)
      parts.add('EuiccRspCapability.euiccRspCapInInfo1');
    if ((value & osUpdateSupport) != 0)
      parts.add('EuiccRspCapability.osUpdateSupport');
    if ((value & cancelForEmptySpnPnSupport) != 0)
      parts.add('EuiccRspCapability.cancelForEmptySpnPnSupport');
    if ((value & updateNotifConfigInfoSupport) != 0)
      parts.add('EuiccRspCapability.updateNotifConfigInfoSupport');
    if ((value & updateMetadataV3Support) != 0)
      parts.add('EuiccRspCapability.updateMetadataV3Support');
    if ((value & v3ObjectsInCtxParamsCASupport) != 0)
      parts.add('EuiccRspCapability.v3ObjectsInCtxParamsCASupport');
    if ((value & pushServiceRegistrationSupport) != 0)
      parts.add('EuiccRspCapability.pushServiceRegistrationSupport');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

enum LpaMode {
  lpad, // 0
  lpae, // 1
}

extension LpaModeExt on LpaMode {
  int get value {
    switch (this) {
      case LpaMode.lpad:
        return 0;
      case LpaMode.lpae:
        return 1;
    }
  }

  static LpaMode? fromValue(int v) {
    switch (v) {
      case 0:
        return LpaMode.lpad;
      case 1:
        return LpaMode.lpae;
      default:
        return null;
    }
  }
}

enum UpdateMetadataResponse {
  ok, // 0
  enterpriseConfigurationNotAllowed, // 6
  commandError, // 7
  pprUpdateInvalidSetting, // 12
  invalidRpmConfiguration, // 14
  deleteNotAllowed, // 15
  undefinedError, // 127
}

extension UpdateMetadataResponseExt on UpdateMetadataResponse {
  int get value {
    switch (this) {
      case UpdateMetadataResponse.ok:
        return 0;
      case UpdateMetadataResponse.enterpriseConfigurationNotAllowed:
        return 6;
      case UpdateMetadataResponse.commandError:
        return 7;
      case UpdateMetadataResponse.pprUpdateInvalidSetting:
        return 12;
      case UpdateMetadataResponse.invalidRpmConfiguration:
        return 14;
      case UpdateMetadataResponse.deleteNotAllowed:
        return 15;
      case UpdateMetadataResponse.undefinedError:
        return 127;
    }
  }

  static UpdateMetadataResponse? fromValue(int v) {
    switch (v) {
      case 0:
        return UpdateMetadataResponse.ok;
      case 6:
        return UpdateMetadataResponse.enterpriseConfigurationNotAllowed;
      case 7:
        return UpdateMetadataResponse.commandError;
      case 12:
        return UpdateMetadataResponse.pprUpdateInvalidSetting;
      case 14:
        return UpdateMetadataResponse.invalidRpmConfiguration;
      case 15:
        return UpdateMetadataResponse.deleteNotAllowed;
      case 127:
        return UpdateMetadataResponse.undefinedError;
      default:
        return null;
    }
  }
}

class NotificationEvent {
  final int value;
  const NotificationEvent(this.value);

  static const int notificationInstall = 1 << 0;
  static const int notificationLocalEnable = 1 << 1;
  static const int notificationLocalDisable = 1 << 2;
  static const int notificationLocalDelete = 1 << 3;
  static const int notificationRpmEnable = 1 << 4;
  static const int notificationRpmDisable = 1 << 5;
  static const int notificationRpmDelete = 1 << 6;
  static const int loadRpmPackageResult = 1 << 7;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & notificationInstall) != 0)
      parts.add('NotificationEvent.notificationInstall');
    if ((value & notificationLocalEnable) != 0)
      parts.add('NotificationEvent.notificationLocalEnable');
    if ((value & notificationLocalDisable) != 0)
      parts.add('NotificationEvent.notificationLocalDisable');
    if ((value & notificationLocalDelete) != 0)
      parts.add('NotificationEvent.notificationLocalDelete');
    if ((value & notificationRpmEnable) != 0)
      parts.add('NotificationEvent.notificationRpmEnable');
    if ((value & notificationRpmDisable) != 0)
      parts.add('NotificationEvent.notificationRpmDisable');
    if ((value & notificationRpmDelete) != 0)
      parts.add('NotificationEvent.notificationRpmDelete');
    if ((value & loadRpmPackageResult) != 0)
      parts.add('NotificationEvent.loadRpmPackageResult');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

class ServiceDescription {
  final int value;
  const ServiceDescription(this.value);

  static const int voice = 1 << 0;
  static const int data = 1 << 1;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & voice) != 0) parts.add('ServiceDescription.voice');
    if ((value & data) != 0) parts.add('ServiceDescription.data');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

class OperationType {
  final int value;
  const OperationType(this.value);

  static const int profileDownload = 1 << 0;
  static const int rpm = 1 << 1;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & profileDownload) != 0)
      parts.add('OperationType.profileDownload');
    if ((value & rpm) != 0) parts.add('OperationType.rpm');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

class ServerRspCapability {
  final int value;
  const ServerRspCapability(this.value);

  static const int crlStaplingV3Support = 1 << 0;
  static const int eventListSigningV3Support = 1 << 1;
  static const int pushServiceV3Support = 1 << 2;
  static const int cancelForEmptySpnPnSupport = 1 << 3;
  static const int cancelForSessionAbortedSupport = 1 << 4;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & crlStaplingV3Support) != 0)
      parts.add('ServerRspCapability.crlStaplingV3Support');
    if ((value & eventListSigningV3Support) != 0)
      parts.add('ServerRspCapability.eventListSigningV3Support');
    if ((value & pushServiceV3Support) != 0)
      parts.add('ServerRspCapability.pushServiceV3Support');
    if ((value & cancelForEmptySpnPnSupport) != 0)
      parts.add('ServerRspCapability.cancelForEmptySpnPnSupport');
    if ((value & cancelForSessionAbortedSupport) != 0)
      parts.add('ServerRspCapability.cancelForSessionAbortedSupport');
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

enum AuthenticateErrorCode {
  invalidCertificate, // 1
  invalidSignature, // 2
  unsupportedCurve, // 3
  noSession, // 4
  invalidOid, // 5
  euiccChallengeMismatch, // 6
  ciPKUnknown, // 7
  transactionIdError, // 8
  missingCrl, // 9
  invalidCrlSignature, // 10
  revokedCert, // 11
  invalidCertOrCrlTime, // 12
  invalidCertOrCrlConfiguration, // 13
  invalidIccid, // 14
  undefinedError, // 127
}

extension AuthenticateErrorCodeExt on AuthenticateErrorCode {
  int get value {
    switch (this) {
      case AuthenticateErrorCode.invalidCertificate:
        return 1;
      case AuthenticateErrorCode.invalidSignature:
        return 2;
      case AuthenticateErrorCode.unsupportedCurve:
        return 3;
      case AuthenticateErrorCode.noSession:
        return 4;
      case AuthenticateErrorCode.invalidOid:
        return 5;
      case AuthenticateErrorCode.euiccChallengeMismatch:
        return 6;
      case AuthenticateErrorCode.ciPKUnknown:
        return 7;
      case AuthenticateErrorCode.transactionIdError:
        return 8;
      case AuthenticateErrorCode.missingCrl:
        return 9;
      case AuthenticateErrorCode.invalidCrlSignature:
        return 10;
      case AuthenticateErrorCode.revokedCert:
        return 11;
      case AuthenticateErrorCode.invalidCertOrCrlTime:
        return 12;
      case AuthenticateErrorCode.invalidCertOrCrlConfiguration:
        return 13;
      case AuthenticateErrorCode.invalidIccid:
        return 14;
      case AuthenticateErrorCode.undefinedError:
        return 127;
    }
  }

  static AuthenticateErrorCode? fromValue(int v) {
    switch (v) {
      case 1:
        return AuthenticateErrorCode.invalidCertificate;
      case 2:
        return AuthenticateErrorCode.invalidSignature;
      case 3:
        return AuthenticateErrorCode.unsupportedCurve;
      case 4:
        return AuthenticateErrorCode.noSession;
      case 5:
        return AuthenticateErrorCode.invalidOid;
      case 6:
        return AuthenticateErrorCode.euiccChallengeMismatch;
      case 7:
        return AuthenticateErrorCode.ciPKUnknown;
      case 8:
        return AuthenticateErrorCode.transactionIdError;
      case 9:
        return AuthenticateErrorCode.missingCrl;
      case 10:
        return AuthenticateErrorCode.invalidCrlSignature;
      case 11:
        return AuthenticateErrorCode.revokedCert;
      case 12:
        return AuthenticateErrorCode.invalidCertOrCrlTime;
      case 13:
        return AuthenticateErrorCode.invalidCertOrCrlConfiguration;
      case 14:
        return AuthenticateErrorCode.invalidIccid;
      case 127:
        return AuthenticateErrorCode.undefinedError;
      default:
        return null;
    }
  }
}

enum CancelSessionReason {
  endUserRejection, // 0
  postponed, // 1
  timeout, // 2
  pprNotAllowed, // 3
  metadataMismatch, // 4
  loadBppExecutionError, // 5
  sessionAborted, // 16
  enterpriseProfilesNotSupported, // 17
  enterpriseRulesNotAllowed, // 18
  enterpriseProfileNotAllowed, // 19
  enterpriseOidMismatch, // 20
  enterpriseRulesError, // 21
  enterpriseProfilesOnly, // 22
  lprNotSupported, // 23
  lprNetworkDataNotAllowed, // 24
  emptyProfileOrSpName, // 25
  rpmDisabled, // 27
  invalidRpmPackage, // 28
  loadRpmPackageError, // 29
  operationAbandoned, // 30
  undefinedReason, // 127
}

extension CancelSessionReasonExt on CancelSessionReason {
  int get value {
    switch (this) {
      case CancelSessionReason.endUserRejection:
        return 0;
      case CancelSessionReason.postponed:
        return 1;
      case CancelSessionReason.timeout:
        return 2;
      case CancelSessionReason.pprNotAllowed:
        return 3;
      case CancelSessionReason.metadataMismatch:
        return 4;
      case CancelSessionReason.loadBppExecutionError:
        return 5;
      case CancelSessionReason.sessionAborted:
        return 16;
      case CancelSessionReason.enterpriseProfilesNotSupported:
        return 17;
      case CancelSessionReason.enterpriseRulesNotAllowed:
        return 18;
      case CancelSessionReason.enterpriseProfileNotAllowed:
        return 19;
      case CancelSessionReason.enterpriseOidMismatch:
        return 20;
      case CancelSessionReason.enterpriseRulesError:
        return 21;
      case CancelSessionReason.enterpriseProfilesOnly:
        return 22;
      case CancelSessionReason.lprNotSupported:
        return 23;
      case CancelSessionReason.lprNetworkDataNotAllowed:
        return 24;
      case CancelSessionReason.emptyProfileOrSpName:
        return 25;
      case CancelSessionReason.rpmDisabled:
        return 27;
      case CancelSessionReason.invalidRpmPackage:
        return 28;
      case CancelSessionReason.loadRpmPackageError:
        return 29;
      case CancelSessionReason.operationAbandoned:
        return 30;
      case CancelSessionReason.undefinedReason:
        return 127;
    }
  }

  static CancelSessionReason? fromValue(int v) {
    switch (v) {
      case 0:
        return CancelSessionReason.endUserRejection;
      case 1:
        return CancelSessionReason.postponed;
      case 2:
        return CancelSessionReason.timeout;
      case 3:
        return CancelSessionReason.pprNotAllowed;
      case 4:
        return CancelSessionReason.metadataMismatch;
      case 5:
        return CancelSessionReason.loadBppExecutionError;
      case 16:
        return CancelSessionReason.sessionAborted;
      case 17:
        return CancelSessionReason.enterpriseProfilesNotSupported;
      case 18:
        return CancelSessionReason.enterpriseRulesNotAllowed;
      case 19:
        return CancelSessionReason.enterpriseProfileNotAllowed;
      case 20:
        return CancelSessionReason.enterpriseOidMismatch;
      case 21:
        return CancelSessionReason.enterpriseRulesError;
      case 22:
        return CancelSessionReason.enterpriseProfilesOnly;
      case 23:
        return CancelSessionReason.lprNotSupported;
      case 24:
        return CancelSessionReason.lprNetworkDataNotAllowed;
      case 25:
        return CancelSessionReason.emptyProfileOrSpName;
      case 27:
        return CancelSessionReason.rpmDisabled;
      case 28:
        return CancelSessionReason.invalidRpmPackage;
      case 29:
        return CancelSessionReason.loadRpmPackageError;
      case 30:
        return CancelSessionReason.operationAbandoned;
      case 127:
        return CancelSessionReason.undefinedReason;
      default:
        return null;
    }
  }
}

enum CancelSessionResponse_cancelSessionResponseError {
  invalidTransactionId, // 5
  undefinedError, // 127
}

extension CancelSessionResponse_cancelSessionResponseErrorExt
    on CancelSessionResponse_cancelSessionResponseError {
  int get value {
    switch (this) {
      case CancelSessionResponse_cancelSessionResponseError
          .invalidTransactionId:
        return 5;
      case CancelSessionResponse_cancelSessionResponseError.undefinedError:
        return 127;
    }
  }

  static CancelSessionResponse_cancelSessionResponseError? fromValue(int v) {
    switch (v) {
      case 5:
        return CancelSessionResponse_cancelSessionResponseError
            .invalidTransactionId;
      case 127:
        return CancelSessionResponse_cancelSessionResponseError.undefinedError;
      default:
        return null;
    }
  }
}

enum IconType {
  jpg, // 0
  png, // 1
}

extension IconTypeExt on IconType {
  int get value {
    switch (this) {
      case IconType.jpg:
        return 0;
      case IconType.png:
        return 1;
    }
  }

  static IconType? fromValue(int v) {
    switch (v) {
      case 0:
        return IconType.jpg;
      case 1:
        return IconType.png;
      default:
        return null;
    }
  }
}

enum ProfileState {
  disabled, // 0
  enabled, // 1
}

extension ProfileStateExt on ProfileState {
  int get value {
    switch (this) {
      case ProfileState.disabled:
        return 0;
      case ProfileState.enabled:
        return 1;
    }
  }

  static ProfileState? fromValue(int v) {
    switch (v) {
      case 0:
        return ProfileState.disabled;
      case 1:
        return ProfileState.enabled;
      default:
        return null;
    }
  }
}

enum ProfileClass {
  test, // 0
  provisioning, // 1
  operational, // 2
}

extension ProfileClassExt on ProfileClass {
  int get value {
    switch (this) {
      case ProfileClass.test:
        return 0;
      case ProfileClass.provisioning:
        return 1;
      case ProfileClass.operational:
        return 2;
    }
  }

  static ProfileClass? fromValue(int v) {
    switch (v) {
      case 0:
        return ProfileClass.test;
      case 1:
        return ProfileClass.provisioning;
      case 2:
        return ProfileClass.operational;
      default:
        return null;
    }
  }
}

enum ProfileInfoListError {
  incorrectInputValues, // 1
  profileChangeOngoing, // 11
  undefinedError, // 127
}

extension ProfileInfoListErrorExt on ProfileInfoListError {
  int get value {
    switch (this) {
      case ProfileInfoListError.incorrectInputValues:
        return 1;
      case ProfileInfoListError.profileChangeOngoing:
        return 11;
      case ProfileInfoListError.undefinedError:
        return 127;
    }
  }

  static ProfileInfoListError? fromValue(int v) {
    switch (v) {
      case 1:
        return ProfileInfoListError.incorrectInputValues;
      case 11:
        return ProfileInfoListError.profileChangeOngoing;
      case 127:
        return ProfileInfoListError.undefinedError;
      default:
        return null;
    }
  }
}

enum EnableProfileResponse_enableResult {
  ok, // 0
  iccidOrAidNotFound, // 1
  profileNotInDisabledState, // 2
  disallowedByPolicy, // 3
  wrongProfileReenabling, // 4
  catBusy, // 5
  disallowedByEnterpriseRule, // 6
  commandError, // 7
  disallowedForRpm, // 9
  noEsimPortAvailable, // 10
  undefinedError, // 127
}

extension EnableProfileResponse_enableResultExt
    on EnableProfileResponse_enableResult {
  int get value {
    switch (this) {
      case EnableProfileResponse_enableResult.ok:
        return 0;
      case EnableProfileResponse_enableResult.iccidOrAidNotFound:
        return 1;
      case EnableProfileResponse_enableResult.profileNotInDisabledState:
        return 2;
      case EnableProfileResponse_enableResult.disallowedByPolicy:
        return 3;
      case EnableProfileResponse_enableResult.wrongProfileReenabling:
        return 4;
      case EnableProfileResponse_enableResult.catBusy:
        return 5;
      case EnableProfileResponse_enableResult.disallowedByEnterpriseRule:
        return 6;
      case EnableProfileResponse_enableResult.commandError:
        return 7;
      case EnableProfileResponse_enableResult.disallowedForRpm:
        return 9;
      case EnableProfileResponse_enableResult.noEsimPortAvailable:
        return 10;
      case EnableProfileResponse_enableResult.undefinedError:
        return 127;
    }
  }

  static EnableProfileResponse_enableResult? fromValue(int v) {
    switch (v) {
      case 0:
        return EnableProfileResponse_enableResult.ok;
      case 1:
        return EnableProfileResponse_enableResult.iccidOrAidNotFound;
      case 2:
        return EnableProfileResponse_enableResult.profileNotInDisabledState;
      case 3:
        return EnableProfileResponse_enableResult.disallowedByPolicy;
      case 4:
        return EnableProfileResponse_enableResult.wrongProfileReenabling;
      case 5:
        return EnableProfileResponse_enableResult.catBusy;
      case 6:
        return EnableProfileResponse_enableResult.disallowedByEnterpriseRule;
      case 7:
        return EnableProfileResponse_enableResult.commandError;
      case 9:
        return EnableProfileResponse_enableResult.disallowedForRpm;
      case 10:
        return EnableProfileResponse_enableResult.noEsimPortAvailable;
      case 127:
        return EnableProfileResponse_enableResult.undefinedError;
      default:
        return null;
    }
  }
}

enum DisableProfileResponse_disableResult {
  ok, // 0
  iccidOrAidNotFound, // 1
  profileNotInEnabledState, // 2
  disallowedByPolicy, // 3
  catBusy, // 5
  disallowedByEnterpriseRule, // 6
  commandError, // 7
  disallowedForRpm, // 9
  undefinedError, // 127
}

extension DisableProfileResponse_disableResultExt
    on DisableProfileResponse_disableResult {
  int get value {
    switch (this) {
      case DisableProfileResponse_disableResult.ok:
        return 0;
      case DisableProfileResponse_disableResult.iccidOrAidNotFound:
        return 1;
      case DisableProfileResponse_disableResult.profileNotInEnabledState:
        return 2;
      case DisableProfileResponse_disableResult.disallowedByPolicy:
        return 3;
      case DisableProfileResponse_disableResult.catBusy:
        return 5;
      case DisableProfileResponse_disableResult.disallowedByEnterpriseRule:
        return 6;
      case DisableProfileResponse_disableResult.commandError:
        return 7;
      case DisableProfileResponse_disableResult.disallowedForRpm:
        return 9;
      case DisableProfileResponse_disableResult.undefinedError:
        return 127;
    }
  }

  static DisableProfileResponse_disableResult? fromValue(int v) {
    switch (v) {
      case 0:
        return DisableProfileResponse_disableResult.ok;
      case 1:
        return DisableProfileResponse_disableResult.iccidOrAidNotFound;
      case 2:
        return DisableProfileResponse_disableResult.profileNotInEnabledState;
      case 3:
        return DisableProfileResponse_disableResult.disallowedByPolicy;
      case 5:
        return DisableProfileResponse_disableResult.catBusy;
      case 6:
        return DisableProfileResponse_disableResult.disallowedByEnterpriseRule;
      case 7:
        return DisableProfileResponse_disableResult.commandError;
      case 9:
        return DisableProfileResponse_disableResult.disallowedForRpm;
      case 127:
        return DisableProfileResponse_disableResult.undefinedError;
      default:
        return null;
    }
  }
}

enum DeleteProfileResponse_deleteResult {
  ok, // 0
  iccidOrAidNotFound, // 1
  profileNotInDisabledState, // 2
  disallowedByPolicy, // 3
  disallowedInTestMode, // 4
  commandError, // 7
  undefinedError, // 127
}

extension DeleteProfileResponse_deleteResultExt
    on DeleteProfileResponse_deleteResult {
  int get value {
    switch (this) {
      case DeleteProfileResponse_deleteResult.ok:
        return 0;
      case DeleteProfileResponse_deleteResult.iccidOrAidNotFound:
        return 1;
      case DeleteProfileResponse_deleteResult.profileNotInDisabledState:
        return 2;
      case DeleteProfileResponse_deleteResult.disallowedByPolicy:
        return 3;
      case DeleteProfileResponse_deleteResult.disallowedInTestMode:
        return 4;
      case DeleteProfileResponse_deleteResult.commandError:
        return 7;
      case DeleteProfileResponse_deleteResult.undefinedError:
        return 127;
    }
  }

  static DeleteProfileResponse_deleteResult? fromValue(int v) {
    switch (v) {
      case 0:
        return DeleteProfileResponse_deleteResult.ok;
      case 1:
        return DeleteProfileResponse_deleteResult.iccidOrAidNotFound;
      case 2:
        return DeleteProfileResponse_deleteResult.profileNotInDisabledState;
      case 3:
        return DeleteProfileResponse_deleteResult.disallowedByPolicy;
      case 4:
        return DeleteProfileResponse_deleteResult.disallowedInTestMode;
      case 7:
        return DeleteProfileResponse_deleteResult.commandError;
      case 127:
        return DeleteProfileResponse_deleteResult.undefinedError;
      default:
        return null;
    }
  }
}

class EuiccMemoryResetRequest_resetOptions {
  final int value;
  const EuiccMemoryResetRequest_resetOptions(this.value);

  static const int deleteOperationalProfiles = 1 << 0;
  static const int deleteFieldLoadedTestProfiles = 1 << 1;
  static const int resetDefaultSmdpAddress = 1 << 2;
  static const int deletePreLoadedTestProfiles = 1 << 3;
  static const int deleteProvisioningProfiles = 1 << 4;

  @override
  String toString() {
    final parts = <String>[];
    if ((value & deleteOperationalProfiles) != 0)
      parts.add(
        'EuiccMemoryResetRequest_resetOptions.deleteOperationalProfiles',
      );
    if ((value & deleteFieldLoadedTestProfiles) != 0)
      parts.add(
        'EuiccMemoryResetRequest_resetOptions.deleteFieldLoadedTestProfiles',
      );
    if ((value & resetDefaultSmdpAddress) != 0)
      parts.add('EuiccMemoryResetRequest_resetOptions.resetDefaultSmdpAddress');
    if ((value & deletePreLoadedTestProfiles) != 0)
      parts.add(
        'EuiccMemoryResetRequest_resetOptions.deletePreLoadedTestProfiles',
      );
    if ((value & deleteProvisioningProfiles) != 0)
      parts.add(
        'EuiccMemoryResetRequest_resetOptions.deleteProvisioningProfiles',
      );
    if (parts.isEmpty && value != 0)
      return '0x${value.toRadixString(16).toUpperCase()}';
    return parts.join(' | ');
  }
}

enum EuiccMemoryResetResponse_resetResult {
  ok, // 0
  nothingToDelete, // 1
  catBusy, // 5
  undefinedError, // 127
}

extension EuiccMemoryResetResponse_resetResultExt
    on EuiccMemoryResetResponse_resetResult {
  int get value {
    switch (this) {
      case EuiccMemoryResetResponse_resetResult.ok:
        return 0;
      case EuiccMemoryResetResponse_resetResult.nothingToDelete:
        return 1;
      case EuiccMemoryResetResponse_resetResult.catBusy:
        return 5;
      case EuiccMemoryResetResponse_resetResult.undefinedError:
        return 127;
    }
  }

  static EuiccMemoryResetResponse_resetResult? fromValue(int v) {
    switch (v) {
      case 0:
        return EuiccMemoryResetResponse_resetResult.ok;
      case 1:
        return EuiccMemoryResetResponse_resetResult.nothingToDelete;
      case 5:
        return EuiccMemoryResetResponse_resetResult.catBusy;
      case 127:
        return EuiccMemoryResetResponse_resetResult.undefinedError;
      default:
        return null;
    }
  }
}

enum SetNicknameResponse_setNicknameResult {
  ok, // 0
  iccidNotFound, // 1
  undefinedError, // 127
}

extension SetNicknameResponse_setNicknameResultExt
    on SetNicknameResponse_setNicknameResult {
  int get value {
    switch (this) {
      case SetNicknameResponse_setNicknameResult.ok:
        return 0;
      case SetNicknameResponse_setNicknameResult.iccidNotFound:
        return 1;
      case SetNicknameResponse_setNicknameResult.undefinedError:
        return 127;
    }
  }

  static SetNicknameResponse_setNicknameResult? fromValue(int v) {
    switch (v) {
      case 0:
        return SetNicknameResponse_setNicknameResult.ok;
      case 1:
        return SetNicknameResponse_setNicknameResult.iccidNotFound;
      case 127:
        return SetNicknameResponse_setNicknameResult.undefinedError;
      default:
        return null;
    }
  }
}

class AttributeTypeAndValue {
  final AttributeType? type;
  final AttributeValue? value;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AttributeTypeAndValue._({this.type, this.value, this.asn1});

  const AttributeTypeAndValue({AttributeType? type, AttributeValue? value})
    : this._(type: type, value: value, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (type != null)
      children.add(_asn1EncodeRaw(0xC, Uint8List.fromList(type!.codeUnits)));
    if (value != null) children.add(_asn1EncodeRaw(0x4, value!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AttributeTypeAndValue.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    AttributeType? type;
    AttributeValue? value;
    int childIdx = 0;
    int found;
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      type = String.fromCharCodes(child.value);
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      value = child.value;
      childIdx = found + 1;
    }
    return AttributeTypeAndValue._(
      type: type,
      value: value,
      asn1: node, // Store original node
    );
  }

  static AttributeTypeAndValue decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return AttributeTypeAndValue.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return AttributeTypeAndValue.fromAsn1(node, tag: tag);
    }
    return AttributeTypeAndValue();
  }

  @override
  String toString() {
    String t = type ?? 'unknown';
    if (t == "\x55\x04\x03")
      t = "CN";
    else if (t == "\x55\x04\x06")
      t = "C";
    else if (t == "\x55\x04\x0A")
      t = "O";
    else if (t == "\x55\x04\x0B")
      t = "OU";
    else if (t == "\x55\x04\x07")
      t = "L";
    else if (t == "\x55\x04\x08")
      t = "ST";
    String v = 'null';
    final val = value;
    if (val != null) {
      try {
        v = utf8.decode(val);
      } catch (_) {
        v = val.map((b) => b.toRadixString(16).padLeft(2, '0')).join('');
      }
    }
    return '$t=$v';
  }
}

class Name {
  final RDNSequence? rdnSequence;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const Name._({this.rdnSequence, this.asn1});

  const Name({RDNSequence? rdnSequence})
    : this._(rdnSequence: rdnSequence, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (rdnSequence != null) {
      final items = <Uint8List>[];
      for (final v in rdnSequence!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0x30, items));
    }
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory Name.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    RDNSequence? rdnSequence;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0x30:
        final items = <List<AttributeTypeAndValue>>[];
        for (final n in node.children) {
          items.add(
            n.children.map((sn0) => AttributeTypeAndValue.decode(sn0)).toList(),
          );
        }
        rdnSequence = items;
        break;
    }
    return Name._(
      rdnSequence: rdnSequence,
      asn1: node, // Store original node
    );
  }

  static Name decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return Name.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return Name.fromAsn1(node, tag: tag);
    }
    return Name();
  }

  @override
  String toString() {
    if (rdnSequence == null) return 'Name()';
    return rdnSequence!
        .map((item) {
          if (item is List) {
            return item.map((e) => e.toString()).join(" + ");
          }
          return item.toString();
        })
        .join(", ");
  }
}

class Certificate {
  final TBSCertificate? tbsCertificate;
  final AlgorithmIdentifier? signatureAlgorithm;
  final int? signature;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const Certificate._({
    this.tbsCertificate,
    this.signatureAlgorithm,
    this.signature,
    this.asn1,
  });

  const Certificate({
    TBSCertificate? tbsCertificate,
    AlgorithmIdentifier? signatureAlgorithm,
    int? signature,
  }) : this._(
         tbsCertificate: tbsCertificate,
         signatureAlgorithm: signatureAlgorithm,
         signature: signature,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (tbsCertificate != null)
      children.add((tbsCertificate as dynamic).encode(tag: 0x30));
    if (signatureAlgorithm != null)
      children.add((signatureAlgorithm as dynamic).encode(tag: 0x30));
    if (signature != null)
      children.add(_asn1EncodeRaw(0x2, _asn1EncodeInt(signature!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory Certificate.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TBSCertificate? tbsCertificate;
    AlgorithmIdentifier? signatureAlgorithm;
    int? signature;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      tbsCertificate = TBSCertificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      signatureAlgorithm = AlgorithmIdentifier.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x2);
    if (found >= 0) {
      final child = node!.children[found];
      signature = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return Certificate._(
      tbsCertificate: tbsCertificate,
      signatureAlgorithm: signatureAlgorithm,
      signature: signature,
      asn1: node, // Store original node
    );
  }

  static Certificate decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return Certificate.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return Certificate.fromAsn1(node, tag: tag);
    }
    return Certificate();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('Certificate(');
    final tbsCertificateVal = tbsCertificate;
    if (tbsCertificateVal != null) {
      final val = tbsCertificateVal;
      if (val is Uint8List) {
        buffer.writeln('  tbsCertificate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tbsCertificate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final signatureAlgorithmVal = signatureAlgorithm;
    if (signatureAlgorithmVal != null) {
      final val = signatureAlgorithmVal;
      if (val is Uint8List) {
        buffer.writeln('  signatureAlgorithm: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  signatureAlgorithm: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final signatureVal = signature;
    if (signatureVal != null) {
      final val = signatureVal;
      if (val is Uint8List) {
        buffer.writeln('  signature: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  signature: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class TBSCertificate {
  final Version? version;
  final CertificateSerialNumber? serialNumber;
  final AlgorithmIdentifier? signature;
  final Name? issuer;
  final Validity? validity;
  final Name? subject;
  final SubjectPublicKeyInfo? subjectPublicKeyInfo;
  final int? issuerUniqueID;
  final int? subjectUniqueID;
  final Extensions? extensions;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const TBSCertificate._({
    this.version,
    this.serialNumber,
    this.signature,
    this.issuer,
    this.validity,
    this.subject,
    this.subjectPublicKeyInfo,
    this.issuerUniqueID,
    this.subjectUniqueID,
    this.extensions,
    this.asn1,
  });

  const TBSCertificate({
    Version? version,
    CertificateSerialNumber? serialNumber,
    AlgorithmIdentifier? signature,
    Name? issuer,
    Validity? validity,
    Name? subject,
    SubjectPublicKeyInfo? subjectPublicKeyInfo,
    int? issuerUniqueID,
    int? subjectUniqueID,
    Extensions? extensions,
  }) : this._(
         version: version,
         serialNumber: serialNumber,
         signature: signature,
         issuer: issuer,
         validity: validity,
         subject: subject,
         subjectPublicKeyInfo: subjectPublicKeyInfo,
         issuerUniqueID: issuerUniqueID,
         subjectUniqueID: subjectUniqueID,
         extensions: extensions,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (version != null)
      children.add(_asn1EncodeRaw(0xA0, _asn1EncodeInt(version!.value)));
    if (serialNumber != null) children.add(_asn1EncodeRaw(0x4, serialNumber!));
    if (signature != null)
      children.add((signature as dynamic).encode(tag: 0x30));
    if (issuer != null) children.add((issuer as dynamic).encode());
    if (validity != null) children.add((validity as dynamic).encode(tag: 0x30));
    if (subject != null) children.add((subject as dynamic).encode());
    if (subjectPublicKeyInfo != null)
      children.add((subjectPublicKeyInfo as dynamic).encode(tag: 0x30));
    if (issuerUniqueID != null)
      children.add(_asn1EncodeRaw(0x81, _asn1EncodeBits(issuerUniqueID!)));
    if (subjectUniqueID != null)
      children.add(_asn1EncodeRaw(0x82, _asn1EncodeBits(subjectUniqueID!)));
    if (extensions != null) {
      final items = <Uint8List>[];
      for (final v in extensions!) {
        items.add(_asn1EncodeConstructed(0x30, [(v as dynamic).encode()]));
      }
      children.add(_asn1EncodeConstructed(0xA3, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory TBSCertificate.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Version? version;
    CertificateSerialNumber? serialNumber;
    AlgorithmIdentifier? signature;
    Name? issuer;
    Validity? validity;
    Name? subject;
    SubjectPublicKeyInfo? subjectPublicKeyInfo;
    int? issuerUniqueID;
    int? subjectUniqueID;
    Extensions? extensions;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      version = VersionExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      serialNumber = child.value;
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      signature = AlgorithmIdentifier.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      issuer = Name.decode(child, tag: child.tag);
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      validity = Validity.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      subject = Name.decode(child, tag: child.tag);
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      subjectPublicKeyInfo = SubjectPublicKeyInfo.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      issuerUniqueID = _asn1DecodeBits(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      subjectUniqueID = _asn1DecodeBits(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA3);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Extension>[];
      for (final n in child.children) {
        items.add(Extension.decode(n));
      }
      extensions = items;
      childIdx = found + 1;
    }
    return TBSCertificate._(
      version: version,
      serialNumber: serialNumber,
      signature: signature,
      issuer: issuer,
      validity: validity,
      subject: subject,
      subjectPublicKeyInfo: subjectPublicKeyInfo,
      issuerUniqueID: issuerUniqueID,
      subjectUniqueID: subjectUniqueID,
      extensions: extensions,
      asn1: node, // Store original node
    );
  }

  static TBSCertificate decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return TBSCertificate.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return TBSCertificate.fromAsn1(node, tag: tag);
    }
    return TBSCertificate();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('TBSCertificate(');
    final versionVal = version;
    if (versionVal != null) {
      final val = versionVal;
      if (val is Uint8List) {
        buffer.writeln('  version: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  version: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serialNumberVal = serialNumber;
    if (serialNumberVal != null) {
      final val = serialNumberVal;
      if (val is Uint8List) {
        buffer.writeln('  serialNumber: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serialNumber: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final signatureVal = signature;
    if (signatureVal != null) {
      final val = signatureVal;
      if (val is Uint8List) {
        buffer.writeln('  signature: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  signature: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final issuerVal = issuer;
    if (issuerVal != null) {
      final val = issuerVal;
      if (val is Uint8List) {
        buffer.writeln('  issuer: ${_hex(val)},');
      } else {
        buffer.writeln('  issuer: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final validityVal = validity;
    if (validityVal != null) {
      final val = validityVal;
      if (val is Uint8List) {
        buffer.writeln('  validity: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  validity: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final subjectVal = subject;
    if (subjectVal != null) {
      final val = subjectVal;
      if (val is Uint8List) {
        buffer.writeln('  subject: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  subject: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final subjectPublicKeyInfoVal = subjectPublicKeyInfo;
    if (subjectPublicKeyInfoVal != null) {
      final val = subjectPublicKeyInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  subjectPublicKeyInfo: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  subjectPublicKeyInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final issuerUniqueIDVal = issuerUniqueID;
    if (issuerUniqueIDVal != null) {
      final val = issuerUniqueIDVal;
      if (val is Uint8List) {
        buffer.writeln('  issuerUniqueID: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  issuerUniqueID: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final subjectUniqueIDVal = subjectUniqueID;
    if (subjectUniqueIDVal != null) {
      final val = subjectUniqueIDVal;
      if (val is Uint8List) {
        buffer.writeln('  subjectUniqueID: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  subjectUniqueID: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final extensionsVal = extensions;
    if (extensionsVal != null) {
      final val = extensionsVal;
      if (val is Uint8List) {
        buffer.writeln('  extensions: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  extensions: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  extensions: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class Validity {
  final Time? notBefore;
  final Time? notAfter;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const Validity._({this.notBefore, this.notAfter, this.asn1});

  const Validity({Time? notBefore, Time? notAfter})
    : this._(notBefore: notBefore, notAfter: notAfter, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (notBefore != null) children.add((notBefore as dynamic).encode());
    if (notAfter != null) children.add((notAfter as dynamic).encode());
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory Validity.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Time? notBefore;
    Time? notAfter;
    int childIdx = 0;
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      notBefore = Time.decode(child, tag: child.tag);
      childIdx++;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      notAfter = Time.decode(child, tag: child.tag);
      childIdx++;
    }
    return Validity._(
      notBefore: notBefore,
      notAfter: notAfter,
      asn1: node, // Store original node
    );
  }

  static Validity decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return Validity.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return Validity.fromAsn1(node, tag: tag);
    }
    return Validity();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('Validity(');
    final notBeforeVal = notBefore;
    if (notBeforeVal != null) {
      final val = notBeforeVal;
      if (val is Uint8List) {
        buffer.writeln('  notBefore: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  notBefore: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notAfterVal = notAfter;
    if (notAfterVal != null) {
      final val = notAfterVal;
      if (val is Uint8List) {
        buffer.writeln('  notAfter: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  notAfter: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class Time {
  final String? utcTime;
  final String? generalTime;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const Time._({this.utcTime, this.generalTime, this.asn1});

  const Time({String? utcTime, String? generalTime})
    : this._(utcTime: utcTime, generalTime: generalTime, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (utcTime != null)
      children.add(
        _asn1EncodeRaw(0x17, Uint8List.fromList(utcTime!.codeUnits)),
      );
    if (generalTime != null)
      children.add(
        _asn1EncodeRaw(0x18, Uint8List.fromList(generalTime!.codeUnits)),
      );
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory Time.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    String? utcTime;
    String? generalTime;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0x17:
        utcTime = String.fromCharCodes(node.value);
        break;
      case 0x18:
        generalTime = String.fromCharCodes(node.value);
        break;
    }
    return Time._(
      utcTime: utcTime,
      generalTime: generalTime,
      asn1: node, // Store original node
    );
  }

  static Time decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return Time.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return Time.fromAsn1(node, tag: tag);
    }
    return Time();
  }

  @override
  String toString() {
    if (utcTime != null) return utcTime!;
    if (generalTime != null) return generalTime!;
    return 'Time()';
  }
}

class SubjectPublicKeyInfo {
  final AlgorithmIdentifier? algorithm;
  final int? subjectPublicKey;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SubjectPublicKeyInfo._({
    this.algorithm,
    this.subjectPublicKey,
    this.asn1,
  });

  const SubjectPublicKeyInfo({
    AlgorithmIdentifier? algorithm,
    int? subjectPublicKey,
  }) : this._(
         algorithm: algorithm,
         subjectPublicKey: subjectPublicKey,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (algorithm != null)
      children.add((algorithm as dynamic).encode(tag: 0x30));
    if (subjectPublicKey != null)
      children.add(_asn1EncodeRaw(0x2, _asn1EncodeInt(subjectPublicKey!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SubjectPublicKeyInfo.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    AlgorithmIdentifier? algorithm;
    int? subjectPublicKey;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      algorithm = AlgorithmIdentifier.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x2);
    if (found >= 0) {
      final child = node!.children[found];
      subjectPublicKey = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return SubjectPublicKeyInfo._(
      algorithm: algorithm,
      subjectPublicKey: subjectPublicKey,
      asn1: node, // Store original node
    );
  }

  static SubjectPublicKeyInfo decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return SubjectPublicKeyInfo.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return SubjectPublicKeyInfo.fromAsn1(node, tag: tag);
    }
    return SubjectPublicKeyInfo();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SubjectPublicKeyInfo(');
    final algorithmVal = algorithm;
    if (algorithmVal != null) {
      final val = algorithmVal;
      if (val is Uint8List) {
        buffer.writeln('  algorithm: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  algorithm: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final subjectPublicKeyVal = subjectPublicKey;
    if (subjectPublicKeyVal != null) {
      final val = subjectPublicKeyVal;
      if (val is Uint8List) {
        buffer.writeln('  subjectPublicKey: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  subjectPublicKey: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class Extension {
  final Uint8List? extnID;
  final bool? critical;
  final Uint8List? extnValue;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const Extension._({this.extnID, this.critical, this.extnValue, this.asn1});

  const Extension({Uint8List? extnID, bool? critical, Uint8List? extnValue})
    : this._(
        extnID: extnID,
        critical: critical,
        extnValue: extnValue,
        asn1: null,
      );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (extnID != null) children.add(_asn1EncodeRaw(0x4, extnID!));
    if (critical != null)
      children.add(
        _asn1EncodeRaw(0x1, Uint8List.fromList([critical! ? 0xFF : 0x00])),
      );
    if (extnValue != null) children.add(_asn1EncodeRaw(0x4, extnValue!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory Extension.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? extnID;
    bool? critical;
    Uint8List? extnValue;
    int childIdx = 0;
    int found;
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      extnID = child.value;
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0x1);
    if (found >= 0) {
      final child = node!.children[found];
      critical = child.value.isNotEmpty && child.value.first != 0;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      extnValue = child.value;
      childIdx = found + 1;
    }
    return Extension._(
      extnID: extnID,
      critical: critical,
      extnValue: extnValue,
      asn1: node, // Store original node
    );
  }

  static Extension decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return Extension.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return Extension.fromAsn1(node, tag: tag);
    }
    return Extension();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('Extension(');
    final extnIDVal = extnID;
    if (extnIDVal != null) {
      final val = extnIDVal;
      if (val is Uint8List) {
        buffer.writeln('  extnID: ${_hex(val)},');
      } else {
        buffer.writeln('  extnID: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final criticalVal = critical;
    if (criticalVal != null) {
      final val = criticalVal;
      if (val is Uint8List) {
        buffer.writeln('  critical: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  critical: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final extnValueVal = extnValue;
    if (extnValueVal != null) {
      final val = extnValueVal;
      if (val is Uint8List) {
        buffer.writeln('  extnValue: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  extnValue: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CertificateList {
  final TBSCertList? tbsCertList;
  final AlgorithmIdentifier? signatureAlgorithm;
  final int? signature;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CertificateList._({
    this.tbsCertList,
    this.signatureAlgorithm,
    this.signature,
    this.asn1,
  });

  const CertificateList({
    TBSCertList? tbsCertList,
    AlgorithmIdentifier? signatureAlgorithm,
    int? signature,
  }) : this._(
         tbsCertList: tbsCertList,
         signatureAlgorithm: signatureAlgorithm,
         signature: signature,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (tbsCertList != null)
      children.add((tbsCertList as dynamic).encode(tag: 0x30));
    if (signatureAlgorithm != null)
      children.add((signatureAlgorithm as dynamic).encode(tag: 0x30));
    if (signature != null)
      children.add(_asn1EncodeRaw(0x2, _asn1EncodeInt(signature!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CertificateList.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TBSCertList? tbsCertList;
    AlgorithmIdentifier? signatureAlgorithm;
    int? signature;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      tbsCertList = TBSCertList.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      signatureAlgorithm = AlgorithmIdentifier.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x2);
    if (found >= 0) {
      final child = node!.children[found];
      signature = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return CertificateList._(
      tbsCertList: tbsCertList,
      signatureAlgorithm: signatureAlgorithm,
      signature: signature,
      asn1: node, // Store original node
    );
  }

  static CertificateList decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return CertificateList.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return CertificateList.fromAsn1(node, tag: tag);
    }
    return CertificateList();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CertificateList(');
    final tbsCertListVal = tbsCertList;
    if (tbsCertListVal != null) {
      final val = tbsCertListVal;
      if (val is Uint8List) {
        buffer.writeln('  tbsCertList: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tbsCertList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final signatureAlgorithmVal = signatureAlgorithm;
    if (signatureAlgorithmVal != null) {
      final val = signatureAlgorithmVal;
      if (val is Uint8List) {
        buffer.writeln('  signatureAlgorithm: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  signatureAlgorithm: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final signatureVal = signature;
    if (signatureVal != null) {
      final val = signatureVal;
      if (val is Uint8List) {
        buffer.writeln('  signature: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  signature: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class TBSCertList {
  final Version? version;
  final AlgorithmIdentifier? signature;
  final Name? issuer;
  final Time? thisUpdate;
  final Time? nextUpdate;
  final List<TBSCertList_revokedCertificates__item_>? revokedCertificates;
  final Extensions? crlExtensions;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const TBSCertList._({
    this.version,
    this.signature,
    this.issuer,
    this.thisUpdate,
    this.nextUpdate,
    this.revokedCertificates,
    this.crlExtensions,
    this.asn1,
  });

  const TBSCertList({
    Version? version,
    AlgorithmIdentifier? signature,
    Name? issuer,
    Time? thisUpdate,
    Time? nextUpdate,
    List<TBSCertList_revokedCertificates__item_>? revokedCertificates,
    Extensions? crlExtensions,
  }) : this._(
         version: version,
         signature: signature,
         issuer: issuer,
         thisUpdate: thisUpdate,
         nextUpdate: nextUpdate,
         revokedCertificates: revokedCertificates,
         crlExtensions: crlExtensions,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (version != null)
      children.add(_asn1EncodeRaw(0x2, _asn1EncodeInt(version!.value)));
    if (signature != null)
      children.add((signature as dynamic).encode(tag: 0x30));
    if (issuer != null) children.add((issuer as dynamic).encode());
    if (thisUpdate != null) children.add((thisUpdate as dynamic).encode());
    if (nextUpdate != null) children.add((nextUpdate as dynamic).encode());
    if (revokedCertificates != null) {
      final items = <Uint8List>[];
      for (final v in revokedCertificates!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0x30, items));
    }
    if (crlExtensions != null) {
      final items = <Uint8List>[];
      for (final v in crlExtensions!) {
        items.add(_asn1EncodeConstructed(0x30, [(v as dynamic).encode()]));
      }
      children.add(_asn1EncodeConstructed(0xA0, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory TBSCertList.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Version? version;
    AlgorithmIdentifier? signature;
    Name? issuer;
    Time? thisUpdate;
    Time? nextUpdate;
    List<TBSCertList_revokedCertificates__item_>? revokedCertificates;
    Extensions? crlExtensions;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x2);
    if (found >= 0) {
      final child = node!.children[found];
      version = VersionExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      signature = AlgorithmIdentifier.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      issuer = Name.decode(child, tag: child.tag);
      childIdx++;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      thisUpdate = Time.decode(child, tag: child.tag);
      childIdx++;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      nextUpdate = Time.decode(child, tag: child.tag);
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <TBSCertList_revokedCertificates__item_>[];
      for (final n in child.children) {
        items.add(TBSCertList_revokedCertificates__item_.decode(n));
      }
      revokedCertificates = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Extension>[];
      for (final n in child.children) {
        items.add(Extension.decode(n));
      }
      crlExtensions = items;
      childIdx = found + 1;
    }
    return TBSCertList._(
      version: version,
      signature: signature,
      issuer: issuer,
      thisUpdate: thisUpdate,
      nextUpdate: nextUpdate,
      revokedCertificates: revokedCertificates,
      crlExtensions: crlExtensions,
      asn1: node, // Store original node
    );
  }

  static TBSCertList decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return TBSCertList.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return TBSCertList.fromAsn1(node, tag: tag);
    }
    return TBSCertList();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('TBSCertList(');
    final versionVal = version;
    if (versionVal != null) {
      final val = versionVal;
      if (val is Uint8List) {
        buffer.writeln('  version: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  version: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final signatureVal = signature;
    if (signatureVal != null) {
      final val = signatureVal;
      if (val is Uint8List) {
        buffer.writeln('  signature: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  signature: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final issuerVal = issuer;
    if (issuerVal != null) {
      final val = issuerVal;
      if (val is Uint8List) {
        buffer.writeln('  issuer: ${_hex(val)},');
      } else {
        buffer.writeln('  issuer: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final thisUpdateVal = thisUpdate;
    if (thisUpdateVal != null) {
      final val = thisUpdateVal;
      if (val is Uint8List) {
        buffer.writeln('  thisUpdate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  thisUpdate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final nextUpdateVal = nextUpdate;
    if (nextUpdateVal != null) {
      final val = nextUpdateVal;
      if (val is Uint8List) {
        buffer.writeln('  nextUpdate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  nextUpdate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final revokedCertificatesVal = revokedCertificates;
    if (revokedCertificatesVal != null) {
      final val = revokedCertificatesVal;
      if (val is Uint8List) {
        buffer.writeln('  revokedCertificates: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  revokedCertificates: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  revokedCertificates: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final crlExtensionsVal = crlExtensions;
    if (crlExtensionsVal != null) {
      final val = crlExtensionsVal;
      if (val is Uint8List) {
        buffer.writeln('  crlExtensions: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  crlExtensions: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  crlExtensions: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class TBSCertList_revokedCertificates__item_ {
  final CertificateSerialNumber? userCertificate;
  final Time? revocationDate;
  final Extensions? crlEntryExtensions;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const TBSCertList_revokedCertificates__item_._({
    this.userCertificate,
    this.revocationDate,
    this.crlEntryExtensions,
    this.asn1,
  });

  const TBSCertList_revokedCertificates__item_({
    CertificateSerialNumber? userCertificate,
    Time? revocationDate,
    Extensions? crlEntryExtensions,
  }) : this._(
         userCertificate: userCertificate,
         revocationDate: revocationDate,
         crlEntryExtensions: crlEntryExtensions,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (userCertificate != null)
      children.add(_asn1EncodeRaw(0x4, userCertificate!));
    if (revocationDate != null)
      children.add((revocationDate as dynamic).encode());
    if (crlEntryExtensions != null) {
      final items = <Uint8List>[];
      for (final v in crlEntryExtensions!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0x30, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory TBSCertList_revokedCertificates__item_.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0x30;
    CertificateSerialNumber? userCertificate;
    Time? revocationDate;
    Extensions? crlEntryExtensions;
    int childIdx = 0;
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      userCertificate = child.value;
      childIdx++;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      revocationDate = Time.decode(child, tag: child.tag);
      childIdx++;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      final items = <Extension>[];
      for (final n in child.children) {
        items.add(Extension.decode(n));
      }
      crlEntryExtensions = items;
      childIdx++;
    }
    return TBSCertList_revokedCertificates__item_._(
      userCertificate: userCertificate,
      revocationDate: revocationDate,
      crlEntryExtensions: crlEntryExtensions,
      asn1: node, // Store original node
    );
  }

  static TBSCertList_revokedCertificates__item_ decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return TBSCertList_revokedCertificates__item_.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return TBSCertList_revokedCertificates__item_.fromAsn1(node, tag: tag);
    }
    return TBSCertList_revokedCertificates__item_();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('TBSCertList_revokedCertificates__item_(');
    final userCertificateVal = userCertificate;
    if (userCertificateVal != null) {
      final val = userCertificateVal;
      if (val is Uint8List) {
        buffer.writeln('  userCertificate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  userCertificate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final revocationDateVal = revocationDate;
    if (revocationDateVal != null) {
      final val = revocationDateVal;
      if (val is Uint8List) {
        buffer.writeln('  revocationDate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  revocationDate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final crlEntryExtensionsVal = crlEntryExtensions;
    if (crlEntryExtensionsVal != null) {
      final val = crlEntryExtensionsVal;
      if (val is Uint8List) {
        buffer.writeln('  crlEntryExtensions: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  crlEntryExtensions: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  crlEntryExtensions: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AlgorithmIdentifier {
  final Uint8List? algorithm;
  final Uint8List? parameters;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AlgorithmIdentifier._({this.algorithm, this.parameters, this.asn1});

  const AlgorithmIdentifier({Uint8List? algorithm, Uint8List? parameters})
    : this._(algorithm: algorithm, parameters: parameters, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (algorithm != null) children.add(_asn1EncodeRaw(0x4, algorithm!));
    if (parameters != null) children.add(_asn1EncodeRaw(0x4, parameters!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AlgorithmIdentifier.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? algorithm;
    Uint8List? parameters;
    int childIdx = 0;
    int found;
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      algorithm = child.value;
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      parameters = child.value;
      childIdx = found + 1;
    }
    return AlgorithmIdentifier._(
      algorithm: algorithm,
      parameters: parameters,
      asn1: node, // Store original node
    );
  }

  static AlgorithmIdentifier decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return AlgorithmIdentifier.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return AlgorithmIdentifier.fromAsn1(node, tag: tag);
    }
    return AlgorithmIdentifier();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AlgorithmIdentifier(');
    final algorithmVal = algorithm;
    if (algorithmVal != null) {
      final val = algorithmVal;
      if (val is Uint8List) {
        buffer.writeln('  algorithm: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  algorithm: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final parametersVal = parameters;
    if (parametersVal != null) {
      final val = parametersVal;
      if (val is Uint8List) {
        buffer.writeln('  parameters: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  parameters: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class OperatorId {
  final Uint8List? mccMnc;
  final Uint8List? gid1;
  final Uint8List? gid2;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const OperatorId._({this.mccMnc, this.gid1, this.gid2, this.asn1});

  const OperatorId({Uint8List? mccMnc, Uint8List? gid1, Uint8List? gid2})
    : this._(mccMnc: mccMnc, gid1: gid1, gid2: gid2, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (mccMnc != null) children.add(_asn1EncodeRaw(0x80, mccMnc!));
    if (gid1 != null) children.add(_asn1EncodeRaw(0x81, gid1!));
    if (gid2 != null) children.add(_asn1EncodeRaw(0x82, gid2!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory OperatorId.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? mccMnc;
    Uint8List? gid1;
    Uint8List? gid2;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      mccMnc = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      gid1 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      gid2 = child.value;
      childIdx = found + 1;
    }
    return OperatorId._(
      mccMnc: mccMnc,
      gid1: gid1,
      gid2: gid2,
      asn1: node, // Store original node
    );
  }

  static OperatorId decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return OperatorId.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return OperatorId.fromAsn1(node, tag: tag);
    }
    return OperatorId();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('OperatorId(');
    final mccMncVal = mccMnc;
    if (mccMncVal != null) {
      final val = mccMncVal;
      if (val is Uint8List) {
        buffer.writeln('  mccMnc: ${_hex(val)},');
      } else {
        buffer.writeln('  mccMnc: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final gid1Val = gid1;
    if (gid1Val != null) {
      final val = gid1Val;
      if (val is Uint8List) {
        buffer.writeln('  gid1: ${_hex(val)},');
      } else {
        buffer.writeln('  gid1: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final gid2Val = gid2;
    if (gid2Val != null) {
      final val = gid2Val;
      if (val is Uint8List) {
        buffer.writeln('  gid2: ${_hex(val)},');
      } else {
        buffer.writeln('  gid2: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmConfiguration {
  final List<RpmConfiguration_managingDpList__item_>? managingDpList;
  final String? pollingAddress;
  final SubjectKeyIdentifier? allowedCiPKId;
  final Uint8List? profileOwnerOid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmConfiguration._({
    this.managingDpList,
    this.pollingAddress,
    this.allowedCiPKId,
    this.profileOwnerOid,
    this.asn1,
  });

  const RpmConfiguration({
    List<RpmConfiguration_managingDpList__item_>? managingDpList,
    String? pollingAddress,
    SubjectKeyIdentifier? allowedCiPKId,
    Uint8List? profileOwnerOid,
  }) : this._(
         managingDpList: managingDpList,
         pollingAddress: pollingAddress,
         allowedCiPKId: allowedCiPKId,
         profileOwnerOid: profileOwnerOid,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (managingDpList != null) {
      final items = <Uint8List>[];
      for (final v in managingDpList!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA0, items));
    }
    if (pollingAddress != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList(pollingAddress!.codeUnits)),
      );
    if (allowedCiPKId != null)
      children.add(_asn1EncodeRaw(0x82, allowedCiPKId!));
    if (profileOwnerOid != null)
      children.add(_asn1EncodeRaw(0x83, profileOwnerOid!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmConfiguration.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    List<RpmConfiguration_managingDpList__item_>? managingDpList;
    String? pollingAddress;
    SubjectKeyIdentifier? allowedCiPKId;
    Uint8List? profileOwnerOid;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <RpmConfiguration_managingDpList__item_>[];
      for (final n in child.children) {
        items.add(RpmConfiguration_managingDpList__item_.decode(n));
      }
      managingDpList = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      pollingAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      allowedCiPKId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      profileOwnerOid = child.value;
      childIdx = found + 1;
    }
    return RpmConfiguration._(
      managingDpList: managingDpList,
      pollingAddress: pollingAddress,
      allowedCiPKId: allowedCiPKId,
      profileOwnerOid: profileOwnerOid,
      asn1: node, // Store original node
    );
  }

  static RpmConfiguration decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return RpmConfiguration.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return RpmConfiguration.fromAsn1(node, tag: tag);
    }
    return RpmConfiguration();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmConfiguration(');
    final managingDpListVal = managingDpList;
    if (managingDpListVal != null) {
      final val = managingDpListVal;
      if (val is Uint8List) {
        buffer.writeln('  managingDpList: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  managingDpList: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  managingDpList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final pollingAddressVal = pollingAddress;
    if (pollingAddressVal != null) {
      final val = pollingAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  pollingAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  pollingAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final allowedCiPKIdVal = allowedCiPKId;
    if (allowedCiPKIdVal != null) {
      final val = allowedCiPKIdVal;
      if (val is Uint8List) {
        buffer.writeln('  allowedCiPKId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  allowedCiPKId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileOwnerOidVal = profileOwnerOid;
    if (profileOwnerOidVal != null) {
      final val = profileOwnerOidVal;
      if (val is Uint8List) {
        buffer.writeln('  profileOwnerOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileOwnerOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmConfiguration_managingDpList__item_ {
  final Uint8List? managingDpOid;
  final RpmType? rpmType;
  final Uint8List? tagList;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmConfiguration_managingDpList__item_._({
    this.managingDpOid,
    this.rpmType,
    this.tagList,
    this.asn1,
  });

  const RpmConfiguration_managingDpList__item_({
    Uint8List? managingDpOid,
    RpmType? rpmType,
    Uint8List? tagList,
  }) : this._(
         managingDpOid: managingDpOid,
         rpmType: rpmType,
         tagList: tagList,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (managingDpOid != null)
      children.add(_asn1EncodeRaw(0x80, managingDpOid!));
    if (rpmType != null)
      children.add(_asn1EncodeRaw(0x81, _asn1EncodeBits(rpmType!.value)));
    if (tagList != null) children.add(_asn1EncodeRaw(0x5C, tagList!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmConfiguration_managingDpList__item_.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0x30;
    Uint8List? managingDpOid;
    RpmType? rpmType;
    Uint8List? tagList;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      managingDpOid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      rpmType = RpmType(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5C);
    if (found >= 0) {
      final child = node!.children[found];
      tagList = child.value;
      childIdx = found + 1;
    }
    return RpmConfiguration_managingDpList__item_._(
      managingDpOid: managingDpOid,
      rpmType: rpmType,
      tagList: tagList,
      asn1: node, // Store original node
    );
  }

  static RpmConfiguration_managingDpList__item_ decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return RpmConfiguration_managingDpList__item_.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RpmConfiguration_managingDpList__item_.fromAsn1(node, tag: tag);
    }
    return RpmConfiguration_managingDpList__item_();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmConfiguration_managingDpList__item_(');
    final managingDpOidVal = managingDpOid;
    if (managingDpOidVal != null) {
      final val = managingDpOidVal;
      if (val is Uint8List) {
        buffer.writeln('  managingDpOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  managingDpOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmTypeVal = rpmType;
    if (rpmTypeVal != null) {
      final val = rpmTypeVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmType: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmType: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final tagListVal = tagList;
    if (tagListVal != null) {
      final val = tagListVal;
      if (val is Uint8List) {
        buffer.writeln('  tagList: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tagList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class LocalisedTextMessage {
  final String? languageTag;
  final String? message;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const LocalisedTextMessage._({this.languageTag, this.message, this.asn1});

  const LocalisedTextMessage({String? languageTag, String? message})
    : this._(languageTag: languageTag, message: message, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (languageTag != null)
      children.add(
        _asn1EncodeRaw(0x80, Uint8List.fromList(languageTag!.codeUnits)),
      );
    if (message != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList(message!.codeUnits)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory LocalisedTextMessage.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    String? languageTag;
    String? message;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      languageTag = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      message = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return LocalisedTextMessage._(
      languageTag: languageTag,
      message: message,
      asn1: node, // Store original node
    );
  }

  static LocalisedTextMessage decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return LocalisedTextMessage.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return LocalisedTextMessage.fromAsn1(node, tag: tag);
    }
    return LocalisedTextMessage();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('LocalisedTextMessage(');
    final languageTagVal = languageTag;
    if (languageTagVal != null) {
      final val = languageTagVal;
      if (val is Uint8List) {
        buffer.writeln('  languageTag: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  languageTag: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final messageVal = message;
    if (messageVal != null) {
      final val = messageVal;
      if (val is Uint8List) {
        buffer.writeln('  message: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  message: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class LprConfiguration {
  final String? pcmpAddress;
  final String? dpiEnable;
  final Uint8List? triggerLprOnEnableProfile;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const LprConfiguration._({
    this.pcmpAddress,
    this.dpiEnable,
    this.triggerLprOnEnableProfile,
    this.asn1,
  });

  const LprConfiguration({
    String? pcmpAddress,
    String? dpiEnable,
    Uint8List? triggerLprOnEnableProfile,
  }) : this._(
         pcmpAddress: pcmpAddress,
         dpiEnable: dpiEnable,
         triggerLprOnEnableProfile: triggerLprOnEnableProfile,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (pcmpAddress != null)
      children.add(
        _asn1EncodeRaw(0x80, Uint8List.fromList(pcmpAddress!.codeUnits)),
      );
    if (dpiEnable != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList(dpiEnable!.codeUnits)),
      );
    if (triggerLprOnEnableProfile != null)
      children.add(_asn1EncodeRaw(0x82, triggerLprOnEnableProfile!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory LprConfiguration.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    String? pcmpAddress;
    String? dpiEnable;
    Uint8List? triggerLprOnEnableProfile;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      pcmpAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      dpiEnable = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      triggerLprOnEnableProfile = child.value;
      childIdx = found + 1;
    }
    return LprConfiguration._(
      pcmpAddress: pcmpAddress,
      dpiEnable: dpiEnable,
      triggerLprOnEnableProfile: triggerLprOnEnableProfile,
      asn1: node, // Store original node
    );
  }

  static LprConfiguration decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return LprConfiguration.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return LprConfiguration.fromAsn1(node, tag: tag);
    }
    return LprConfiguration();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('LprConfiguration(');
    final pcmpAddressVal = pcmpAddress;
    if (pcmpAddressVal != null) {
      final val = pcmpAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  pcmpAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  pcmpAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final dpiEnableVal = dpiEnable;
    if (dpiEnableVal != null) {
      final val = dpiEnableVal;
      if (val is Uint8List) {
        buffer.writeln('  dpiEnable: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  dpiEnable: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final triggerLprOnEnableProfileVal = triggerLprOnEnableProfile;
    if (triggerLprOnEnableProfileVal != null) {
      final val = triggerLprOnEnableProfileVal;
      if (val is Uint8List) {
        buffer.writeln('  triggerLprOnEnableProfile: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  triggerLprOnEnableProfile: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EnterpriseConfiguration {
  final Uint8List? enterpriseOid;
  final String? enterpriseName;
  final EnterpriseConfiguration_enterpriseRules? enterpriseRules;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EnterpriseConfiguration._({
    this.enterpriseOid,
    this.enterpriseName,
    this.enterpriseRules,
    this.asn1,
  });

  const EnterpriseConfiguration({
    Uint8List? enterpriseOid,
    String? enterpriseName,
    EnterpriseConfiguration_enterpriseRules? enterpriseRules,
  }) : this._(
         enterpriseOid: enterpriseOid,
         enterpriseName: enterpriseName,
         enterpriseRules: enterpriseRules,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (enterpriseOid != null)
      children.add(_asn1EncodeRaw(0x80, enterpriseOid!));
    if (enterpriseName != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList(enterpriseName!.codeUnits)),
      );
    if (enterpriseRules != null)
      children.add((enterpriseRules as dynamic).encode(tag: 0xA2));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EnterpriseConfiguration.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? enterpriseOid;
    String? enterpriseName;
    EnterpriseConfiguration_enterpriseRules? enterpriseRules;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      enterpriseOid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      enterpriseName = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA2);
    if (found >= 0) {
      final child = node!.children[found];
      enterpriseRules = EnterpriseConfiguration_enterpriseRules.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    return EnterpriseConfiguration._(
      enterpriseOid: enterpriseOid,
      enterpriseName: enterpriseName,
      enterpriseRules: enterpriseRules,
      asn1: node, // Store original node
    );
  }

  static EnterpriseConfiguration decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return EnterpriseConfiguration.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return EnterpriseConfiguration.fromAsn1(node, tag: tag);
    }
    return EnterpriseConfiguration();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EnterpriseConfiguration(');
    final enterpriseOidVal = enterpriseOid;
    if (enterpriseOidVal != null) {
      final val = enterpriseOidVal;
      if (val is Uint8List) {
        buffer.writeln('  enterpriseOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enterpriseOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final enterpriseNameVal = enterpriseName;
    if (enterpriseNameVal != null) {
      final val = enterpriseNameVal;
      if (val is Uint8List) {
        buffer.writeln('  enterpriseName: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enterpriseName: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final enterpriseRulesVal = enterpriseRules;
    if (enterpriseRulesVal != null) {
      final val = enterpriseRulesVal;
      if (val is Uint8List) {
        buffer.writeln('  enterpriseRules: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enterpriseRules: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EnterpriseConfiguration_enterpriseRules {
  final int? enterpriseRuleBits;
  final int? numberOfNonEnterpriseProfiles;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EnterpriseConfiguration_enterpriseRules._({
    this.enterpriseRuleBits,
    this.numberOfNonEnterpriseProfiles,
    this.asn1,
  });

  const EnterpriseConfiguration_enterpriseRules({
    int? enterpriseRuleBits,
    int? numberOfNonEnterpriseProfiles,
  }) : this._(
         enterpriseRuleBits: enterpriseRuleBits,
         numberOfNonEnterpriseProfiles: numberOfNonEnterpriseProfiles,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA2;
    if (enterpriseRuleBits != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(enterpriseRuleBits!)));
    if (numberOfNonEnterpriseProfiles != null)
      children.add(
        _asn1EncodeRaw(0x81, _asn1EncodeInt(numberOfNonEnterpriseProfiles!)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EnterpriseConfiguration_enterpriseRules.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA2;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int? enterpriseRuleBits;
    int? numberOfNonEnterpriseProfiles;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      enterpriseRuleBits = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      numberOfNonEnterpriseProfiles = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return EnterpriseConfiguration_enterpriseRules._(
      enterpriseRuleBits: enterpriseRuleBits,
      numberOfNonEnterpriseProfiles: numberOfNonEnterpriseProfiles,
      asn1: node, // Store original node
    );
  }

  static EnterpriseConfiguration_enterpriseRules decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return EnterpriseConfiguration_enterpriseRules.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return EnterpriseConfiguration_enterpriseRules.fromAsn1(node, tag: tag);
    }
    return EnterpriseConfiguration_enterpriseRules();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EnterpriseConfiguration_enterpriseRules(');
    final enterpriseRuleBitsVal = enterpriseRuleBits;
    if (enterpriseRuleBitsVal != null) {
      final val = enterpriseRuleBitsVal;
      if (val is Uint8List) {
        buffer.writeln('  enterpriseRuleBits: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enterpriseRuleBits: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final numberOfNonEnterpriseProfilesVal = numberOfNonEnterpriseProfiles;
    if (numberOfNonEnterpriseProfilesVal != null) {
      final val = numberOfNonEnterpriseProfilesVal;
      if (val is Uint8List) {
        buffer.writeln('  numberOfNonEnterpriseProfiles: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  numberOfNonEnterpriseProfiles: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class VendorSpecificExtension__item_ {
  final Uint8List? vendorOid;
  final Uint8List? vendorSpecificData;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const VendorSpecificExtension__item_._({
    this.vendorOid,
    this.vendorSpecificData,
    this.asn1,
  });

  const VendorSpecificExtension__item_({
    Uint8List? vendorOid,
    Uint8List? vendorSpecificData,
  }) : this._(
         vendorOid: vendorOid,
         vendorSpecificData: vendorSpecificData,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (vendorOid != null) children.add(_asn1EncodeRaw(0x80, vendorOid!));
    if (vendorSpecificData != null)
      children.add(_asn1EncodeRaw(0x81, vendorSpecificData!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory VendorSpecificExtension__item_.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? vendorOid;
    Uint8List? vendorSpecificData;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      vendorOid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      vendorSpecificData = child.value;
      childIdx = found + 1;
    }
    return VendorSpecificExtension__item_._(
      vendorOid: vendorOid,
      vendorSpecificData: vendorSpecificData,
      asn1: node, // Store original node
    );
  }

  static VendorSpecificExtension__item_ decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return VendorSpecificExtension__item_.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return VendorSpecificExtension__item_.fromAsn1(node, tag: tag);
    }
    return VendorSpecificExtension__item_();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('VendorSpecificExtension__item_(');
    final vendorOidVal = vendorOid;
    if (vendorOidVal != null) {
      final val = vendorOidVal;
      if (val is Uint8List) {
        buffer.writeln('  vendorOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  vendorOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final vendorSpecificDataVal = vendorSpecificData;
    if (vendorSpecificDataVal != null) {
      final val = vendorSpecificDataVal;
      if (val is Uint8List) {
        buffer.writeln('  vendorSpecificData: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  vendorSpecificData: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeviceChangeConfiguration {
  final DeviceChangeConfiguration_requestToDp? requestToDp;
  final DeviceChangeConfiguration_usingStoredAc? usingStoredAc;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeviceChangeConfiguration._({
    this.requestToDp,
    this.usingStoredAc,
    this.asn1,
  });

  const DeviceChangeConfiguration({
    DeviceChangeConfiguration_requestToDp? requestToDp,
    DeviceChangeConfiguration_usingStoredAc? usingStoredAc,
  }) : this._(
         requestToDp: requestToDp,
         usingStoredAc: usingStoredAc,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (requestToDp != null)
      children.add((requestToDp as dynamic).encode(tag: 0xA0));
    if (usingStoredAc != null)
      children.add((usingStoredAc as dynamic).encode(tag: 0xA1));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory DeviceChangeConfiguration.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    DeviceChangeConfiguration_requestToDp? requestToDp;
    DeviceChangeConfiguration_usingStoredAc? usingStoredAc;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0xA0:
        requestToDp = DeviceChangeConfiguration_requestToDp.decode(
          node,
          tag: 0xA0,
        );
        break;
      case 0xA1:
        usingStoredAc = DeviceChangeConfiguration_usingStoredAc.decode(
          node,
          tag: 0xA1,
        );
        break;
    }
    return DeviceChangeConfiguration._(
      requestToDp: requestToDp,
      usingStoredAc: usingStoredAc,
      asn1: node, // Store original node
    );
  }

  static DeviceChangeConfiguration decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return DeviceChangeConfiguration.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return DeviceChangeConfiguration.fromAsn1(node, tag: tag);
    }
    return DeviceChangeConfiguration();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeviceChangeConfiguration(');
    final requestToDpVal = requestToDp;
    if (requestToDpVal != null) {
      final val = requestToDpVal;
      if (val is Uint8List) {
        buffer.writeln('  requestToDp: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  requestToDp: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final usingStoredAcVal = usingStoredAc;
    if (usingStoredAcVal != null) {
      final val = usingStoredAcVal;
      if (val is Uint8List) {
        buffer.writeln('  usingStoredAc: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  usingStoredAc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeviceChangeConfiguration_requestToDp {
  final String? smdpAddressForDc;
  final SubjectKeyIdentifier? allowedCiPKId;
  final Uint8List? eidRequired;
  final Uint8List? tacRequired;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeviceChangeConfiguration_requestToDp._({
    this.smdpAddressForDc,
    this.allowedCiPKId,
    this.eidRequired,
    this.tacRequired,
    this.asn1,
  });

  const DeviceChangeConfiguration_requestToDp({
    String? smdpAddressForDc,
    SubjectKeyIdentifier? allowedCiPKId,
    Uint8List? eidRequired,
    Uint8List? tacRequired,
  }) : this._(
         smdpAddressForDc: smdpAddressForDc,
         allowedCiPKId: allowedCiPKId,
         eidRequired: eidRequired,
         tacRequired: tacRequired,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA0;
    if (smdpAddressForDc != null)
      children.add(
        _asn1EncodeRaw(0x80, Uint8List.fromList(smdpAddressForDc!.codeUnits)),
      );
    if (allowedCiPKId != null)
      children.add(_asn1EncodeRaw(0x81, allowedCiPKId!));
    if (eidRequired != null) children.add(_asn1EncodeRaw(0x82, eidRequired!));
    if (tacRequired != null) children.add(_asn1EncodeRaw(0x83, tacRequired!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DeviceChangeConfiguration_requestToDp.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA0;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    String? smdpAddressForDc;
    SubjectKeyIdentifier? allowedCiPKId;
    Uint8List? eidRequired;
    Uint8List? tacRequired;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      smdpAddressForDc = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      allowedCiPKId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      eidRequired = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      tacRequired = child.value;
      childIdx = found + 1;
    }
    return DeviceChangeConfiguration_requestToDp._(
      smdpAddressForDc: smdpAddressForDc,
      allowedCiPKId: allowedCiPKId,
      eidRequired: eidRequired,
      tacRequired: tacRequired,
      asn1: node, // Store original node
    );
  }

  static DeviceChangeConfiguration_requestToDp decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return DeviceChangeConfiguration_requestToDp.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return DeviceChangeConfiguration_requestToDp.fromAsn1(node, tag: tag);
    }
    return DeviceChangeConfiguration_requestToDp();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeviceChangeConfiguration_requestToDp(');
    final smdpAddressForDcVal = smdpAddressForDc;
    if (smdpAddressForDcVal != null) {
      final val = smdpAddressForDcVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpAddressForDc: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpAddressForDc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final allowedCiPKIdVal = allowedCiPKId;
    if (allowedCiPKIdVal != null) {
      final val = allowedCiPKIdVal;
      if (val is Uint8List) {
        buffer.writeln('  allowedCiPKId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  allowedCiPKId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final eidRequiredVal = eidRequired;
    if (eidRequiredVal != null) {
      final val = eidRequiredVal;
      if (val is Uint8List) {
        buffer.writeln('  eidRequired: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  eidRequired: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final tacRequiredVal = tacRequired;
    if (tacRequiredVal != null) {
      final val = tacRequiredVal;
      if (val is Uint8List) {
        buffer.writeln('  tacRequired: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tacRequired: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeviceChangeConfiguration_usingStoredAc {
  final String? activationCodeForDc;
  final Uint8List? deleteOldProfile;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeviceChangeConfiguration_usingStoredAc._({
    this.activationCodeForDc,
    this.deleteOldProfile,
    this.asn1,
  });

  const DeviceChangeConfiguration_usingStoredAc({
    String? activationCodeForDc,
    Uint8List? deleteOldProfile,
  }) : this._(
         activationCodeForDc: activationCodeForDc,
         deleteOldProfile: deleteOldProfile,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA1;
    if (activationCodeForDc != null)
      children.add(
        _asn1EncodeRaw(
          0x80,
          Uint8List.fromList(activationCodeForDc!.codeUnits),
        ),
      );
    if (deleteOldProfile != null)
      children.add(_asn1EncodeRaw(0x81, deleteOldProfile!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DeviceChangeConfiguration_usingStoredAc.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA1;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    String? activationCodeForDc;
    Uint8List? deleteOldProfile;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      activationCodeForDc = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      deleteOldProfile = child.value;
      childIdx = found + 1;
    }
    return DeviceChangeConfiguration_usingStoredAc._(
      activationCodeForDc: activationCodeForDc,
      deleteOldProfile: deleteOldProfile,
      asn1: node, // Store original node
    );
  }

  static DeviceChangeConfiguration_usingStoredAc decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return DeviceChangeConfiguration_usingStoredAc.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return DeviceChangeConfiguration_usingStoredAc.fromAsn1(node, tag: tag);
    }
    return DeviceChangeConfiguration_usingStoredAc();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeviceChangeConfiguration_usingStoredAc(');
    final activationCodeForDcVal = activationCodeForDc;
    if (activationCodeForDcVal != null) {
      final val = activationCodeForDcVal;
      if (val is Uint8List) {
        buffer.writeln('  activationCodeForDc: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  activationCodeForDc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deleteOldProfileVal = deleteOldProfile;
    if (deleteOldProfileVal != null) {
      final val = deleteOldProfileVal;
      if (val is Uint8List) {
        buffer.writeln('  deleteOldProfile: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deleteOldProfile: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class BoundProfilePackage {
  final InitialiseSecureChannelRequest? initialiseSecureChannelRequest;
  final Uint8List? firstSequenceOf87;
  final List<Uint8List>? sequenceOf88;
  final List<Uint8List>? secondSequenceOf87;
  final List<Uint8List>? sequenceOf86;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const BoundProfilePackage._({
    this.initialiseSecureChannelRequest,
    this.firstSequenceOf87,
    this.sequenceOf88,
    this.secondSequenceOf87,
    this.sequenceOf86,
    this.asn1,
  });

  const BoundProfilePackage({
    InitialiseSecureChannelRequest? initialiseSecureChannelRequest,
    Uint8List? firstSequenceOf87,
    List<Uint8List>? sequenceOf88,
    List<Uint8List>? secondSequenceOf87,
    List<Uint8List>? sequenceOf86,
  }) : this._(
         initialiseSecureChannelRequest: initialiseSecureChannelRequest,
         firstSequenceOf87: firstSequenceOf87,
         sequenceOf88: sequenceOf88,
         secondSequenceOf87: secondSequenceOf87,
         sequenceOf86: sequenceOf86,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF36;
    if (initialiseSecureChannelRequest != null)
      children.add(
        (initialiseSecureChannelRequest as dynamic).encode(tag: 0xBF23),
      );
    if (firstSequenceOf87 != null)
      children.add(_asn1EncodeRaw(0x4, firstSequenceOf87!));
    if (sequenceOf88 != null) {
      final items = <Uint8List>[];
      for (final v in sequenceOf88!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xA1, items));
    }
    if (secondSequenceOf87 != null) {
      final items = <Uint8List>[];
      for (final v in secondSequenceOf87!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xA2, items));
    }
    if (sequenceOf86 != null) {
      final items = <Uint8List>[];
      for (final v in sequenceOf86!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xA3, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory BoundProfilePackage.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF36;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    InitialiseSecureChannelRequest? initialiseSecureChannelRequest;
    Uint8List? firstSequenceOf87;
    List<Uint8List>? sequenceOf88;
    List<Uint8List>? secondSequenceOf87;
    List<Uint8List>? sequenceOf86;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xBF23);
    if (found >= 0) {
      final child = node!.children[found];
      initialiseSecureChannelRequest = InitialiseSecureChannelRequest.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      firstSequenceOf87 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Uint8List>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      sequenceOf88 = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA2);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Uint8List>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      secondSequenceOf87 = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA3);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Uint8List>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      sequenceOf86 = items;
      childIdx = found + 1;
    }
    return BoundProfilePackage._(
      initialiseSecureChannelRequest: initialiseSecureChannelRequest,
      firstSequenceOf87: firstSequenceOf87,
      sequenceOf88: sequenceOf88,
      secondSequenceOf87: secondSequenceOf87,
      sequenceOf86: sequenceOf86,
      asn1: node, // Store original node
    );
  }

  static BoundProfilePackage decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return BoundProfilePackage.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return BoundProfilePackage.fromAsn1(node, tag: tag);
    }
    return BoundProfilePackage();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('BoundProfilePackage(');
    final initialiseSecureChannelRequestVal = initialiseSecureChannelRequest;
    if (initialiseSecureChannelRequestVal != null) {
      final val = initialiseSecureChannelRequestVal;
      if (val is Uint8List) {
        buffer.writeln('  initialiseSecureChannelRequest: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  initialiseSecureChannelRequest: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final firstSequenceOf87Val = firstSequenceOf87;
    if (firstSequenceOf87Val != null) {
      final val = firstSequenceOf87Val;
      if (val is Uint8List) {
        buffer.writeln('  firstSequenceOf87: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  firstSequenceOf87: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final sequenceOf88Val = sequenceOf88;
    if (sequenceOf88Val != null) {
      final val = sequenceOf88Val;
      if (val is Uint8List) {
        buffer.writeln('  sequenceOf88: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  sequenceOf88: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  sequenceOf88: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final secondSequenceOf87Val = secondSequenceOf87;
    if (secondSequenceOf87Val != null) {
      final val = secondSequenceOf87Val;
      if (val is Uint8List) {
        buffer.writeln('  secondSequenceOf87: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  secondSequenceOf87: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  secondSequenceOf87: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final sequenceOf86Val = sequenceOf86;
    if (sequenceOf86Val != null) {
      final val = sequenceOf86Val;
      if (val is Uint8List) {
        buffer.writeln('  sequenceOf86: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  sequenceOf86: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  sequenceOf86: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ProfileInstallationResult {
  final ProfileInstallationResultData? profileInstallationResultData;
  final EuiccSign? euiccSignPIR;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ProfileInstallationResult._({
    this.profileInstallationResultData,
    this.euiccSignPIR,
    this.asn1,
  });

  const ProfileInstallationResult({
    ProfileInstallationResultData? profileInstallationResultData,
    EuiccSign? euiccSignPIR,
  }) : this._(
         profileInstallationResultData: profileInstallationResultData,
         euiccSignPIR: euiccSignPIR,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF37;
    if (profileInstallationResultData != null)
      children.add(
        (profileInstallationResultData as dynamic).encode(tag: 0xBF27),
      );
    if (euiccSignPIR != null)
      children.add(_asn1EncodeRaw(0x5F37, euiccSignPIR!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ProfileInstallationResult.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF37;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    ProfileInstallationResultData? profileInstallationResultData;
    EuiccSign? euiccSignPIR;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xBF27);
    if (found >= 0) {
      final child = node!.children[found];
      profileInstallationResultData = ProfileInstallationResultData.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      euiccSignPIR = child.value;
      childIdx = found + 1;
    }
    return ProfileInstallationResult._(
      profileInstallationResultData: profileInstallationResultData,
      euiccSignPIR: euiccSignPIR,
      asn1: node, // Store original node
    );
  }

  static ProfileInstallationResult decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return ProfileInstallationResult.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return ProfileInstallationResult.fromAsn1(node, tag: tag);
    }
    return ProfileInstallationResult();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ProfileInstallationResult(');
    final profileInstallationResultDataVal = profileInstallationResultData;
    if (profileInstallationResultDataVal != null) {
      final val = profileInstallationResultDataVal;
      if (val is Uint8List) {
        buffer.writeln('  profileInstallationResultData: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileInstallationResultData: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccSignPIRVal = euiccSignPIR;
    if (euiccSignPIRVal != null) {
      final val = euiccSignPIRVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccSignPIR: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccSignPIR: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ProfileInstallationResultData {
  final TransactionId? transactionId;
  final NotificationMetadata? notificationMetadata;
  final Uint8List? smdpOid;
  final ProfileInstallationResultData_finalResult? finalResult;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ProfileInstallationResultData._({
    this.transactionId,
    this.notificationMetadata,
    this.smdpOid,
    this.finalResult,
    this.asn1,
  });

  const ProfileInstallationResultData({
    TransactionId? transactionId,
    NotificationMetadata? notificationMetadata,
    Uint8List? smdpOid,
    ProfileInstallationResultData_finalResult? finalResult,
  }) : this._(
         transactionId: transactionId,
         notificationMetadata: notificationMetadata,
         smdpOid: smdpOid,
         finalResult: finalResult,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF27;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (notificationMetadata != null)
      children.add((notificationMetadata as dynamic).encode(tag: 0xBF2F));
    if (smdpOid != null) children.add(_asn1EncodeRaw(0x4, smdpOid!));
    if (finalResult != null)
      children.add(
        _asn1EncodeConstructed(0xA2, [(finalResult as dynamic).encode()]),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ProfileInstallationResultData.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF27;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    TransactionId? transactionId;
    NotificationMetadata? notificationMetadata;
    Uint8List? smdpOid;
    ProfileInstallationResultData_finalResult? finalResult;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF2F);
    if (found >= 0) {
      final child = node!.children[found];
      notificationMetadata = NotificationMetadata.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      smdpOid = child.value;
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0xA2);
    if (found >= 0) {
      final child = node!.children[found];
      finalResult = ProfileInstallationResultData_finalResult.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    return ProfileInstallationResultData._(
      transactionId: transactionId,
      notificationMetadata: notificationMetadata,
      smdpOid: smdpOid,
      finalResult: finalResult,
      asn1: node, // Store original node
    );
  }

  static ProfileInstallationResultData decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return ProfileInstallationResultData.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return ProfileInstallationResultData.fromAsn1(node, tag: tag);
    }
    return ProfileInstallationResultData();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ProfileInstallationResultData(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notificationMetadataVal = notificationMetadata;
    if (notificationMetadataVal != null) {
      final val = notificationMetadataVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationMetadata: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  notificationMetadata: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpOidVal = smdpOid;
    if (smdpOidVal != null) {
      final val = smdpOidVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final finalResultVal = finalResult;
    if (finalResultVal != null) {
      final val = finalResultVal;
      if (val is Uint8List) {
        buffer.writeln('  finalResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  finalResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ProfileInstallationResultData_finalResult {
  final SuccessResult? successResult;
  final ErrorResult? errorResult;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ProfileInstallationResultData_finalResult._({
    this.successResult,
    this.errorResult,
    this.asn1,
  });

  const ProfileInstallationResultData_finalResult({
    SuccessResult? successResult,
    ErrorResult? errorResult,
  }) : this._(
         successResult: successResult,
         errorResult: errorResult,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA2;
    if (successResult != null)
      children.add((successResult as dynamic).encode(tag: 0xA0));
    if (errorResult != null)
      children.add((errorResult as dynamic).encode(tag: 0xA1));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory ProfileInstallationResultData_finalResult.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA2;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    SuccessResult? successResult;
    ErrorResult? errorResult;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          successResult = SuccessResult.decode(child, tag: 0xA0);
          break;
        case 0xA1:
          errorResult = ErrorResult.decode(child, tag: 0xA1);
          break;
      }
    }
    return ProfileInstallationResultData_finalResult._(
      successResult: successResult,
      errorResult: errorResult,
      asn1: node, // Store original node
    );
  }

  static ProfileInstallationResultData_finalResult decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return ProfileInstallationResultData_finalResult.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return ProfileInstallationResultData_finalResult.fromAsn1(
          node,
          tag: tag,
        );
    }
    return ProfileInstallationResultData_finalResult();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ProfileInstallationResultData_finalResult(');
    final successResultVal = successResult;
    if (successResultVal != null) {
      final val = successResultVal;
      if (val is Uint8List) {
        buffer.writeln('  successResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  successResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final errorResultVal = errorResult;
    if (errorResultVal != null) {
      final val = errorResultVal;
      if (val is Uint8List) {
        buffer.writeln('  errorResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  errorResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SuccessResult {
  final Uint8List? aid;
  final Uint8List? ppiResponse;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SuccessResult._({this.aid, this.ppiResponse, this.asn1});

  const SuccessResult({Uint8List? aid, Uint8List? ppiResponse})
    : this._(aid: aid, ppiResponse: ppiResponse, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (aid != null) children.add(_asn1EncodeRaw(0x4F, aid!));
    if (ppiResponse != null) children.add(_asn1EncodeRaw(0x4, ppiResponse!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SuccessResult.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? aid;
    Uint8List? ppiResponse;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x4F);
    if (found >= 0) {
      final child = node!.children[found];
      aid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      ppiResponse = child.value;
      childIdx = found + 1;
    }
    return SuccessResult._(
      aid: aid,
      ppiResponse: ppiResponse,
      asn1: node, // Store original node
    );
  }

  static SuccessResult decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return SuccessResult.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return SuccessResult.fromAsn1(node, tag: tag);
    }
    return SuccessResult();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SuccessResult(');
    final aidVal = aid;
    if (aidVal != null) {
      final val = aidVal;
      if (val is Uint8List) {
        buffer.writeln('  aid: ${_hex(val)},');
      } else {
        buffer.writeln('  aid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final ppiResponseVal = ppiResponse;
    if (ppiResponseVal != null) {
      final val = ppiResponseVal;
      if (val is Uint8List) {
        buffer.writeln('  ppiResponse: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ppiResponse: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ErrorResult {
  final BppCommandId? bppCommandId;
  final ErrorReason? errorReason;
  final Uint8List? ppiResponse;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ErrorResult._({
    this.bppCommandId,
    this.errorReason,
    this.ppiResponse,
    this.asn1,
  });

  const ErrorResult({
    BppCommandId? bppCommandId,
    ErrorReason? errorReason,
    Uint8List? ppiResponse,
  }) : this._(
         bppCommandId: bppCommandId,
         errorReason: errorReason,
         ppiResponse: ppiResponse,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (bppCommandId != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(bppCommandId!.value)));
    if (errorReason != null)
      children.add(_asn1EncodeRaw(0x81, _asn1EncodeInt(errorReason!.value)));
    if (ppiResponse != null) children.add(_asn1EncodeRaw(0x82, ppiResponse!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ErrorResult.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    BppCommandId? bppCommandId;
    ErrorReason? errorReason;
    Uint8List? ppiResponse;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      bppCommandId = BppCommandIdExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      errorReason = ErrorReasonExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      ppiResponse = child.value;
      childIdx = found + 1;
    }
    return ErrorResult._(
      bppCommandId: bppCommandId,
      errorReason: errorReason,
      ppiResponse: ppiResponse,
      asn1: node, // Store original node
    );
  }

  static ErrorResult decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return ErrorResult.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ErrorResult.fromAsn1(node, tag: tag);
    }
    return ErrorResult();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ErrorResult(');
    final bppCommandIdVal = bppCommandId;
    if (bppCommandIdVal != null) {
      final val = bppCommandIdVal;
      if (val is Uint8List) {
        buffer.writeln('  bppCommandId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  bppCommandId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final errorReasonVal = errorReason;
    if (errorReasonVal != null) {
      final val = errorReasonVal;
      if (val is Uint8List) {
        buffer.writeln('  errorReason: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  errorReason: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ppiResponseVal = ppiResponse;
    if (ppiResponseVal != null) {
      final val = ppiResponseVal;
      if (val is Uint8List) {
        buffer.writeln('  ppiResponse: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ppiResponse: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommand {
  final Uint8List? continueOnFailure;
  final RpmCommand_rpmCommandDetails? rpmCommandDetails;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommand._({
    this.continueOnFailure,
    this.rpmCommandDetails,
    this.asn1,
  });

  const RpmCommand({
    Uint8List? continueOnFailure,
    RpmCommand_rpmCommandDetails? rpmCommandDetails,
  }) : this._(
         continueOnFailure: continueOnFailure,
         rpmCommandDetails: rpmCommandDetails,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (continueOnFailure != null)
      children.add(_asn1EncodeRaw(0x80, continueOnFailure!));
    if (rpmCommandDetails != null)
      children.add((rpmCommandDetails as dynamic).encode());
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmCommand.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? continueOnFailure;
    RpmCommand_rpmCommandDetails? rpmCommandDetails;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      continueOnFailure = child.value;
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      rpmCommandDetails = RpmCommand_rpmCommandDetails.decode(
        child,
        tag: child.tag,
      );
      childIdx++;
    }
    return RpmCommand._(
      continueOnFailure: continueOnFailure,
      rpmCommandDetails: rpmCommandDetails,
      asn1: node, // Store original node
    );
  }

  static RpmCommand decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return RpmCommand.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return RpmCommand.fromAsn1(node, tag: tag);
    }
    return RpmCommand();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommand(');
    final continueOnFailureVal = continueOnFailure;
    if (continueOnFailureVal != null) {
      final val = continueOnFailureVal;
      if (val is Uint8List) {
        buffer.writeln('  continueOnFailure: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  continueOnFailure: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmCommandDetailsVal = rpmCommandDetails;
    if (rpmCommandDetailsVal != null) {
      final val = rpmCommandDetailsVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmCommandDetails: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmCommandDetails: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommand_rpmCommandDetails {
  final RpmCommand_rpmCommandDetails_enable? enable;
  final RpmCommand_rpmCommandDetails_disable? disable;
  final RpmCommand_rpmCommandDetails_delete? delete;
  final ListProfileInfo? listProfileInfo;
  final RpmCommand_rpmCommandDetails_updateMetadata? updateMetadata;
  final RpmCommand_rpmCommandDetails_contactPcmp? contactPcmp;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommand_rpmCommandDetails._({
    this.enable,
    this.disable,
    this.delete,
    this.listProfileInfo,
    this.updateMetadata,
    this.contactPcmp,
    this.asn1,
  });

  const RpmCommand_rpmCommandDetails({
    RpmCommand_rpmCommandDetails_enable? enable,
    RpmCommand_rpmCommandDetails_disable? disable,
    RpmCommand_rpmCommandDetails_delete? delete,
    ListProfileInfo? listProfileInfo,
    RpmCommand_rpmCommandDetails_updateMetadata? updateMetadata,
    RpmCommand_rpmCommandDetails_contactPcmp? contactPcmp,
  }) : this._(
         enable: enable,
         disable: disable,
         delete: delete,
         listProfileInfo: listProfileInfo,
         updateMetadata: updateMetadata,
         contactPcmp: contactPcmp,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (enable != null) children.add((enable as dynamic).encode(tag: 0xA1));
    if (disable != null) children.add((disable as dynamic).encode(tag: 0xA2));
    if (delete != null) children.add((delete as dynamic).encode(tag: 0xA3));
    if (listProfileInfo != null)
      children.add((listProfileInfo as dynamic).encode(tag: 0xA4));
    if (updateMetadata != null)
      children.add((updateMetadata as dynamic).encode(tag: 0xA5));
    if (contactPcmp != null)
      children.add((contactPcmp as dynamic).encode(tag: 0xA6));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory RpmCommand_rpmCommandDetails.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    RpmCommand_rpmCommandDetails_enable? enable;
    RpmCommand_rpmCommandDetails_disable? disable;
    RpmCommand_rpmCommandDetails_delete? delete;
    ListProfileInfo? listProfileInfo;
    RpmCommand_rpmCommandDetails_updateMetadata? updateMetadata;
    RpmCommand_rpmCommandDetails_contactPcmp? contactPcmp;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0xA1:
        enable = RpmCommand_rpmCommandDetails_enable.decode(node, tag: 0xA1);
        break;
      case 0xA2:
        disable = RpmCommand_rpmCommandDetails_disable.decode(node, tag: 0xA2);
        break;
      case 0xA3:
        delete = RpmCommand_rpmCommandDetails_delete.decode(node, tag: 0xA3);
        break;
      case 0xA4:
        listProfileInfo = ListProfileInfo.decode(node, tag: 0xA4);
        break;
      case 0xA5:
        updateMetadata = RpmCommand_rpmCommandDetails_updateMetadata.decode(
          node,
          tag: 0xA5,
        );
        break;
      case 0xA6:
        contactPcmp = RpmCommand_rpmCommandDetails_contactPcmp.decode(
          node,
          tag: 0xA6,
        );
        break;
    }
    return RpmCommand_rpmCommandDetails._(
      enable: enable,
      disable: disable,
      delete: delete,
      listProfileInfo: listProfileInfo,
      updateMetadata: updateMetadata,
      contactPcmp: contactPcmp,
      asn1: node, // Store original node
    );
  }

  static RpmCommand_rpmCommandDetails decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return RpmCommand_rpmCommandDetails.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RpmCommand_rpmCommandDetails.fromAsn1(node, tag: tag);
    }
    return RpmCommand_rpmCommandDetails();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommand_rpmCommandDetails(');
    final enableVal = enable;
    if (enableVal != null) {
      final val = enableVal;
      if (val is Uint8List) {
        buffer.writeln('  enable: ${_hex(val)},');
      } else {
        buffer.writeln('  enable: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final disableVal = disable;
    if (disableVal != null) {
      final val = disableVal;
      if (val is Uint8List) {
        buffer.writeln('  disable: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  disable: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deleteVal = delete;
    if (deleteVal != null) {
      final val = deleteVal;
      if (val is Uint8List) {
        buffer.writeln('  delete: ${_hex(val)},');
      } else {
        buffer.writeln('  delete: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final listProfileInfoVal = listProfileInfo;
    if (listProfileInfoVal != null) {
      final val = listProfileInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  listProfileInfo: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  listProfileInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final updateMetadataVal = updateMetadata;
    if (updateMetadataVal != null) {
      final val = updateMetadataVal;
      if (val is Uint8List) {
        buffer.writeln('  updateMetadata: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  updateMetadata: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final contactPcmpVal = contactPcmp;
    if (contactPcmpVal != null) {
      final val = contactPcmpVal;
      if (val is Uint8List) {
        buffer.writeln('  contactPcmp: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  contactPcmp: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommand_rpmCommandDetails_enable {
  final Iccid? iccid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommand_rpmCommandDetails_enable._({this.iccid, this.asn1});

  const RpmCommand_rpmCommandDetails_enable({Iccid? iccid})
    : this._(iccid: iccid, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA1;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmCommand_rpmCommandDetails_enable.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA1;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    return RpmCommand_rpmCommandDetails_enable._(
      iccid: iccid,
      asn1: node, // Store original node
    );
  }

  static RpmCommand_rpmCommandDetails_enable decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return RpmCommand_rpmCommandDetails_enable.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RpmCommand_rpmCommandDetails_enable.fromAsn1(node, tag: tag);
    }
    return RpmCommand_rpmCommandDetails_enable();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommand_rpmCommandDetails_enable(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommand_rpmCommandDetails_disable {
  final Iccid? iccid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommand_rpmCommandDetails_disable._({this.iccid, this.asn1});

  const RpmCommand_rpmCommandDetails_disable({Iccid? iccid})
    : this._(iccid: iccid, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA2;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmCommand_rpmCommandDetails_disable.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA2;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    return RpmCommand_rpmCommandDetails_disable._(
      iccid: iccid,
      asn1: node, // Store original node
    );
  }

  static RpmCommand_rpmCommandDetails_disable decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return RpmCommand_rpmCommandDetails_disable.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RpmCommand_rpmCommandDetails_disable.fromAsn1(node, tag: tag);
    }
    return RpmCommand_rpmCommandDetails_disable();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommand_rpmCommandDetails_disable(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommand_rpmCommandDetails_delete {
  final Iccid? iccid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommand_rpmCommandDetails_delete._({this.iccid, this.asn1});

  const RpmCommand_rpmCommandDetails_delete({Iccid? iccid})
    : this._(iccid: iccid, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA3;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmCommand_rpmCommandDetails_delete.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA3;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    return RpmCommand_rpmCommandDetails_delete._(
      iccid: iccid,
      asn1: node, // Store original node
    );
  }

  static RpmCommand_rpmCommandDetails_delete decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return RpmCommand_rpmCommandDetails_delete.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RpmCommand_rpmCommandDetails_delete.fromAsn1(node, tag: tag);
    }
    return RpmCommand_rpmCommandDetails_delete();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommand_rpmCommandDetails_delete(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommand_rpmCommandDetails_updateMetadata {
  final Iccid? iccid;
  final UpdateMetadataRequest? updateMetadataRequest;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommand_rpmCommandDetails_updateMetadata._({
    this.iccid,
    this.updateMetadataRequest,
    this.asn1,
  });

  const RpmCommand_rpmCommandDetails_updateMetadata({
    Iccid? iccid,
    UpdateMetadataRequest? updateMetadataRequest,
  }) : this._(
         iccid: iccid,
         updateMetadataRequest: updateMetadataRequest,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA5;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (updateMetadataRequest != null)
      children.add((updateMetadataRequest as dynamic).encode(tag: 0xBF2A));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmCommand_rpmCommandDetails_updateMetadata.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA5;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    UpdateMetadataRequest? updateMetadataRequest;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF2A);
    if (found >= 0) {
      final child = node!.children[found];
      updateMetadataRequest = UpdateMetadataRequest.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    return RpmCommand_rpmCommandDetails_updateMetadata._(
      iccid: iccid,
      updateMetadataRequest: updateMetadataRequest,
      asn1: node, // Store original node
    );
  }

  static RpmCommand_rpmCommandDetails_updateMetadata decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return RpmCommand_rpmCommandDetails_updateMetadata.fromAsn1(
        data,
        tag: tag,
      );
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RpmCommand_rpmCommandDetails_updateMetadata.fromAsn1(
          node,
          tag: tag,
        );
    }
    return RpmCommand_rpmCommandDetails_updateMetadata();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommand_rpmCommandDetails_updateMetadata(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final updateMetadataRequestVal = updateMetadataRequest;
    if (updateMetadataRequestVal != null) {
      final val = updateMetadataRequestVal;
      if (val is Uint8List) {
        buffer.writeln('  updateMetadataRequest: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  updateMetadataRequest: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommand_rpmCommandDetails_contactPcmp {
  final Iccid? iccid;
  final String? dpiRpm;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommand_rpmCommandDetails_contactPcmp._({
    this.iccid,
    this.dpiRpm,
    this.asn1,
  });

  const RpmCommand_rpmCommandDetails_contactPcmp({Iccid? iccid, String? dpiRpm})
    : this._(iccid: iccid, dpiRpm: dpiRpm, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA6;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (dpiRpm != null)
      children.add(_asn1EncodeRaw(0xC, Uint8List.fromList(dpiRpm!.codeUnits)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmCommand_rpmCommandDetails_contactPcmp.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA6;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    String? dpiRpm;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xC);
    if (found >= 0) {
      final child = node!.children[found];
      dpiRpm = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return RpmCommand_rpmCommandDetails_contactPcmp._(
      iccid: iccid,
      dpiRpm: dpiRpm,
      asn1: node, // Store original node
    );
  }

  static RpmCommand_rpmCommandDetails_contactPcmp decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return RpmCommand_rpmCommandDetails_contactPcmp.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RpmCommand_rpmCommandDetails_contactPcmp.fromAsn1(
          node,
          tag: tag,
        );
    }
    return RpmCommand_rpmCommandDetails_contactPcmp();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommand_rpmCommandDetails_contactPcmp(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final dpiRpmVal = dpiRpm;
    if (dpiRpmVal != null) {
      final val = dpiRpmVal;
      if (val is Uint8List) {
        buffer.writeln('  dpiRpm: ${_hex(val)},');
      } else {
        buffer.writeln('  dpiRpm: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class LoadRpmPackageResultSigned {
  final LoadRpmPackageResultDataSigned? loadRpmPackageResultDataSigned;
  final EuiccSign? euiccSignRPR;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const LoadRpmPackageResultSigned._({
    this.loadRpmPackageResultDataSigned,
    this.euiccSignRPR,
    this.asn1,
  });

  const LoadRpmPackageResultSigned({
    LoadRpmPackageResultDataSigned? loadRpmPackageResultDataSigned,
    EuiccSign? euiccSignRPR,
  }) : this._(
         loadRpmPackageResultDataSigned: loadRpmPackageResultDataSigned,
         euiccSignRPR: euiccSignRPR,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (loadRpmPackageResultDataSigned != null)
      children.add(
        (loadRpmPackageResultDataSigned as dynamic).encode(tag: 0xA0),
      );
    if (euiccSignRPR != null) children.add(_asn1EncodeRaw(0x81, euiccSignRPR!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory LoadRpmPackageResultSigned.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    LoadRpmPackageResultDataSigned? loadRpmPackageResultDataSigned;
    EuiccSign? euiccSignRPR;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      loadRpmPackageResultDataSigned = LoadRpmPackageResultDataSigned.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      euiccSignRPR = child.value;
      childIdx = found + 1;
    }
    return LoadRpmPackageResultSigned._(
      loadRpmPackageResultDataSigned: loadRpmPackageResultDataSigned,
      euiccSignRPR: euiccSignRPR,
      asn1: node, // Store original node
    );
  }

  static LoadRpmPackageResultSigned decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return LoadRpmPackageResultSigned.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return LoadRpmPackageResultSigned.fromAsn1(node, tag: tag);
    }
    return LoadRpmPackageResultSigned();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('LoadRpmPackageResultSigned(');
    final loadRpmPackageResultDataSignedVal = loadRpmPackageResultDataSigned;
    if (loadRpmPackageResultDataSignedVal != null) {
      final val = loadRpmPackageResultDataSignedVal;
      if (val is Uint8List) {
        buffer.writeln('  loadRpmPackageResultDataSigned: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  loadRpmPackageResultDataSigned: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccSignRPRVal = euiccSignRPR;
    if (euiccSignRPRVal != null) {
      final val = euiccSignRPRVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccSignRPR: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccSignRPR: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class LoadRpmPackageResultDataSigned {
  final TransactionId? transactionId;
  final NotificationMetadata? notificationMetadata;
  final Uint8List? smdpOid;
  final LoadRpmPackageResultDataSigned_finalResult? finalResult;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const LoadRpmPackageResultDataSigned._({
    this.transactionId,
    this.notificationMetadata,
    this.smdpOid,
    this.finalResult,
    this.asn1,
  });

  const LoadRpmPackageResultDataSigned({
    TransactionId? transactionId,
    NotificationMetadata? notificationMetadata,
    Uint8List? smdpOid,
    LoadRpmPackageResultDataSigned_finalResult? finalResult,
  }) : this._(
         transactionId: transactionId,
         notificationMetadata: notificationMetadata,
         smdpOid: smdpOid,
         finalResult: finalResult,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (notificationMetadata != null)
      children.add((notificationMetadata as dynamic).encode(tag: 0xBF2F));
    if (smdpOid != null) children.add(_asn1EncodeRaw(0x4, smdpOid!));
    if (finalResult != null)
      children.add(
        _asn1EncodeConstructed(0xA2, [(finalResult as dynamic).encode()]),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory LoadRpmPackageResultDataSigned.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    NotificationMetadata? notificationMetadata;
    Uint8List? smdpOid;
    LoadRpmPackageResultDataSigned_finalResult? finalResult;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF2F);
    if (found >= 0) {
      final child = node!.children[found];
      notificationMetadata = NotificationMetadata.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      smdpOid = child.value;
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0xA2);
    if (found >= 0) {
      final child = node!.children[found];
      finalResult = LoadRpmPackageResultDataSigned_finalResult.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    return LoadRpmPackageResultDataSigned._(
      transactionId: transactionId,
      notificationMetadata: notificationMetadata,
      smdpOid: smdpOid,
      finalResult: finalResult,
      asn1: node, // Store original node
    );
  }

  static LoadRpmPackageResultDataSigned decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return LoadRpmPackageResultDataSigned.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return LoadRpmPackageResultDataSigned.fromAsn1(node, tag: tag);
    }
    return LoadRpmPackageResultDataSigned();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('LoadRpmPackageResultDataSigned(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notificationMetadataVal = notificationMetadata;
    if (notificationMetadataVal != null) {
      final val = notificationMetadataVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationMetadata: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  notificationMetadata: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpOidVal = smdpOid;
    if (smdpOidVal != null) {
      final val = smdpOidVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final finalResultVal = finalResult;
    if (finalResultVal != null) {
      final val = finalResultVal;
      if (val is Uint8List) {
        buffer.writeln('  finalResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  finalResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class LoadRpmPackageResultDataSigned_finalResult {
  final List<RpmCommandResult>? rpmPackageExecutionResult;
  final LoadRpmPackageErrorCodeSigned? loadRpmPackageErrorCodeSigned;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const LoadRpmPackageResultDataSigned_finalResult._({
    this.rpmPackageExecutionResult,
    this.loadRpmPackageErrorCodeSigned,
    this.asn1,
  });

  const LoadRpmPackageResultDataSigned_finalResult({
    List<RpmCommandResult>? rpmPackageExecutionResult,
    LoadRpmPackageErrorCodeSigned? loadRpmPackageErrorCodeSigned,
  }) : this._(
         rpmPackageExecutionResult: rpmPackageExecutionResult,
         loadRpmPackageErrorCodeSigned: loadRpmPackageErrorCodeSigned,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA2;
    if (rpmPackageExecutionResult != null) {
      final items = <Uint8List>[];
      for (final v in rpmPackageExecutionResult!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA0, items));
    }
    if (loadRpmPackageErrorCodeSigned != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          _asn1EncodeInt(loadRpmPackageErrorCodeSigned!.value),
        ),
      );
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory LoadRpmPackageResultDataSigned_finalResult.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA2;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    List<RpmCommandResult>? rpmPackageExecutionResult;
    LoadRpmPackageErrorCodeSigned? loadRpmPackageErrorCodeSigned;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          final items = <RpmCommandResult>[];
          for (final n in child.children) {
            items.add(RpmCommandResult.decode(n));
          }
          rpmPackageExecutionResult = items;
          break;
        case 0x81:
          loadRpmPackageErrorCodeSigned =
              LoadRpmPackageErrorCodeSignedExt.fromValue(
                _asn1DecodeInt(child.value),
              );
          break;
      }
    }
    return LoadRpmPackageResultDataSigned_finalResult._(
      rpmPackageExecutionResult: rpmPackageExecutionResult,
      loadRpmPackageErrorCodeSigned: loadRpmPackageErrorCodeSigned,
      asn1: node, // Store original node
    );
  }

  static LoadRpmPackageResultDataSigned_finalResult decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return LoadRpmPackageResultDataSigned_finalResult.fromAsn1(
        data,
        tag: tag,
      );
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return LoadRpmPackageResultDataSigned_finalResult.fromAsn1(
          node,
          tag: tag,
        );
    }
    return LoadRpmPackageResultDataSigned_finalResult();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('LoadRpmPackageResultDataSigned_finalResult(');
    final rpmPackageExecutionResultVal = rpmPackageExecutionResult;
    if (rpmPackageExecutionResultVal != null) {
      final val = rpmPackageExecutionResultVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmPackageExecutionResult: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  rpmPackageExecutionResult: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  rpmPackageExecutionResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final loadRpmPackageErrorCodeSignedVal = loadRpmPackageErrorCodeSigned;
    if (loadRpmPackageErrorCodeSignedVal != null) {
      final val = loadRpmPackageErrorCodeSignedVal;
      if (val is Uint8List) {
        buffer.writeln('  loadRpmPackageErrorCodeSigned: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  loadRpmPackageErrorCodeSigned: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommandResult {
  final Iccid? iccid;
  final RpmCommandResult_rpmCommandResultData? rpmCommandResultData;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommandResult._({this.iccid, this.rpmCommandResultData, this.asn1});

  const RpmCommandResult({
    Iccid? iccid,
    RpmCommandResult_rpmCommandResultData? rpmCommandResultData,
  }) : this._(
         iccid: iccid,
         rpmCommandResultData: rpmCommandResultData,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (rpmCommandResultData != null)
      children.add((rpmCommandResultData as dynamic).encode());
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RpmCommandResult.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Iccid? iccid;
    RpmCommandResult_rpmCommandResultData? rpmCommandResultData;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      rpmCommandResultData = RpmCommandResult_rpmCommandResultData.decode(
        child,
        tag: child.tag,
      );
      childIdx++;
    }
    return RpmCommandResult._(
      iccid: iccid,
      rpmCommandResultData: rpmCommandResultData,
      asn1: node, // Store original node
    );
  }

  static RpmCommandResult decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return RpmCommandResult.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return RpmCommandResult.fromAsn1(node, tag: tag);
    }
    return RpmCommandResult();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommandResult(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final rpmCommandResultDataVal = rpmCommandResultData;
    if (rpmCommandResultDataVal != null) {
      final val = rpmCommandResultDataVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmCommandResultData: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmCommandResultData: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RpmCommandResult_rpmCommandResultData {
  final EnableProfileResponse? enableResult;
  final DisableProfileResponse? disableResult;
  final DeleteProfileResponse? deleteResult;
  final ProfileInfoListResponse? listProfileInfoResult;
  final UpdateMetadataResponse? updateMetadataResult;
  final ContactPcmpResponse? contactPcmpResult;
  final RpmCommandResult_rpmCommandResultData_rpmProcessingTerminated?
  rpmProcessingTerminated;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RpmCommandResult_rpmCommandResultData._({
    this.enableResult,
    this.disableResult,
    this.deleteResult,
    this.listProfileInfoResult,
    this.updateMetadataResult,
    this.contactPcmpResult,
    this.rpmProcessingTerminated,
    this.asn1,
  });

  const RpmCommandResult_rpmCommandResultData({
    EnableProfileResponse? enableResult,
    DisableProfileResponse? disableResult,
    DeleteProfileResponse? deleteResult,
    ProfileInfoListResponse? listProfileInfoResult,
    UpdateMetadataResponse? updateMetadataResult,
    ContactPcmpResponse? contactPcmpResult,
    RpmCommandResult_rpmCommandResultData_rpmProcessingTerminated?
    rpmProcessingTerminated,
  }) : this._(
         enableResult: enableResult,
         disableResult: disableResult,
         deleteResult: deleteResult,
         listProfileInfoResult: listProfileInfoResult,
         updateMetadataResult: updateMetadataResult,
         contactPcmpResult: contactPcmpResult,
         rpmProcessingTerminated: rpmProcessingTerminated,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (enableResult != null)
      children.add((enableResult as dynamic).encode(tag: 0xBF31));
    if (disableResult != null)
      children.add((disableResult as dynamic).encode(tag: 0xBF32));
    if (deleteResult != null)
      children.add((deleteResult as dynamic).encode(tag: 0xBF33));
    if (listProfileInfoResult != null)
      children.add((listProfileInfoResult as dynamic).encode(tag: 0xBF2D));
    if (updateMetadataResult != null)
      children.add(
        _asn1EncodeRaw(0x9F2A, _asn1EncodeInt(updateMetadataResult!.value)),
      );
    if (contactPcmpResult != null)
      children.add(
        _asn1EncodeConstructed(0xA0, [(contactPcmpResult as dynamic).encode()]),
      );
    if (rpmProcessingTerminated != null)
      children.add(
        _asn1EncodeRaw(0x2, _asn1EncodeInt(rpmProcessingTerminated!.value)),
      );
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory RpmCommandResult_rpmCommandResultData.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0x30;
    EnableProfileResponse? enableResult;
    DisableProfileResponse? disableResult;
    DeleteProfileResponse? deleteResult;
    ProfileInfoListResponse? listProfileInfoResult;
    UpdateMetadataResponse? updateMetadataResult;
    ContactPcmpResponse? contactPcmpResult;
    RpmCommandResult_rpmCommandResultData_rpmProcessingTerminated?
    rpmProcessingTerminated;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0xBF31:
        enableResult = EnableProfileResponse.decode(node, tag: 0xBF31);
        break;
      case 0xBF32:
        disableResult = DisableProfileResponse.decode(node, tag: 0xBF32);
        break;
      case 0xBF33:
        deleteResult = DeleteProfileResponse.decode(node, tag: 0xBF33);
        break;
      case 0xBF2D:
        listProfileInfoResult = ProfileInfoListResponse.decode(
          node,
          tag: 0xBF2D,
        );
        break;
      case 0x9F2A:
        updateMetadataResult = UpdateMetadataResponseExt.fromValue(
          _asn1DecodeInt(node.value),
        );
        break;
      case 0xA0:
        contactPcmpResult = ContactPcmpResponse.decode(node.children.first);
        break;
      case 0x2:
        rpmProcessingTerminated =
            RpmCommandResult_rpmCommandResultData_rpmProcessingTerminatedExt.fromValue(
              _asn1DecodeInt(node.value),
            );
        break;
    }
    return RpmCommandResult_rpmCommandResultData._(
      enableResult: enableResult,
      disableResult: disableResult,
      deleteResult: deleteResult,
      listProfileInfoResult: listProfileInfoResult,
      updateMetadataResult: updateMetadataResult,
      contactPcmpResult: contactPcmpResult,
      rpmProcessingTerminated: rpmProcessingTerminated,
      asn1: node, // Store original node
    );
  }

  static RpmCommandResult_rpmCommandResultData decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return RpmCommandResult_rpmCommandResultData.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RpmCommandResult_rpmCommandResultData.fromAsn1(node, tag: tag);
    }
    return RpmCommandResult_rpmCommandResultData();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RpmCommandResult_rpmCommandResultData(');
    final enableResultVal = enableResult;
    if (enableResultVal != null) {
      final val = enableResultVal;
      if (val is Uint8List) {
        buffer.writeln('  enableResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enableResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final disableResultVal = disableResult;
    if (disableResultVal != null) {
      final val = disableResultVal;
      if (val is Uint8List) {
        buffer.writeln('  disableResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  disableResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deleteResultVal = deleteResult;
    if (deleteResultVal != null) {
      final val = deleteResultVal;
      if (val is Uint8List) {
        buffer.writeln('  deleteResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deleteResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final listProfileInfoResultVal = listProfileInfoResult;
    if (listProfileInfoResultVal != null) {
      final val = listProfileInfoResultVal;
      if (val is Uint8List) {
        buffer.writeln('  listProfileInfoResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  listProfileInfoResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final updateMetadataResultVal = updateMetadataResult;
    if (updateMetadataResultVal != null) {
      final val = updateMetadataResultVal;
      if (val is Uint8List) {
        buffer.writeln('  updateMetadataResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  updateMetadataResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final contactPcmpResultVal = contactPcmpResult;
    if (contactPcmpResultVal != null) {
      final val = contactPcmpResultVal;
      if (val is Uint8List) {
        buffer.writeln('  contactPcmpResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  contactPcmpResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmProcessingTerminatedVal = rpmProcessingTerminated;
    if (rpmProcessingTerminatedVal != null) {
      final val = rpmProcessingTerminatedVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmProcessingTerminated: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmProcessingTerminated: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ContactPcmpResponse {
  final ContactPcmpResponse_contactPcmpResponseOk? contactPcmpResponseOk;
  final ContactPcmpResponse_contactPcmpResponseError? contactPcmpResponseError;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ContactPcmpResponse._({
    this.contactPcmpResponseOk,
    this.contactPcmpResponseError,
    this.asn1,
  });

  const ContactPcmpResponse({
    ContactPcmpResponse_contactPcmpResponseOk? contactPcmpResponseOk,
    ContactPcmpResponse_contactPcmpResponseError? contactPcmpResponseError,
  }) : this._(
         contactPcmpResponseOk: contactPcmpResponseOk,
         contactPcmpResponseError: contactPcmpResponseError,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (contactPcmpResponseOk != null)
      children.add((contactPcmpResponseOk as dynamic).encode(tag: 0xA0));
    if (contactPcmpResponseError != null)
      children.add(
        _asn1EncodeRaw(0x81, _asn1EncodeInt(contactPcmpResponseError!.value)),
      );
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory ContactPcmpResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    ContactPcmpResponse_contactPcmpResponseOk? contactPcmpResponseOk;
    ContactPcmpResponse_contactPcmpResponseError? contactPcmpResponseError;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0xA0:
        contactPcmpResponseOk =
            ContactPcmpResponse_contactPcmpResponseOk.decode(node, tag: 0xA0);
        break;
      case 0x81:
        contactPcmpResponseError =
            ContactPcmpResponse_contactPcmpResponseErrorExt.fromValue(
              _asn1DecodeInt(node.value),
            );
        break;
    }
    return ContactPcmpResponse._(
      contactPcmpResponseOk: contactPcmpResponseOk,
      contactPcmpResponseError: contactPcmpResponseError,
      asn1: node, // Store original node
    );
  }

  static ContactPcmpResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return ContactPcmpResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ContactPcmpResponse.fromAsn1(node, tag: tag);
    }
    return ContactPcmpResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ContactPcmpResponse(');
    final contactPcmpResponseOkVal = contactPcmpResponseOk;
    if (contactPcmpResponseOkVal != null) {
      final val = contactPcmpResponseOkVal;
      if (val is Uint8List) {
        buffer.writeln('  contactPcmpResponseOk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  contactPcmpResponseOk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final contactPcmpResponseErrorVal = contactPcmpResponseError;
    if (contactPcmpResponseErrorVal != null) {
      final val = contactPcmpResponseErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  contactPcmpResponseError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  contactPcmpResponseError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ContactPcmpResponse_contactPcmpResponseOk {
  final String? pcmpAddress;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ContactPcmpResponse_contactPcmpResponseOk._({
    this.pcmpAddress,
    this.asn1,
  });

  const ContactPcmpResponse_contactPcmpResponseOk({String? pcmpAddress})
    : this._(pcmpAddress: pcmpAddress, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA0;
    if (pcmpAddress != null)
      children.add(
        _asn1EncodeRaw(0x80, Uint8List.fromList(pcmpAddress!.codeUnits)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ContactPcmpResponse_contactPcmpResponseOk.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA0;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    String? pcmpAddress;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      pcmpAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return ContactPcmpResponse_contactPcmpResponseOk._(
      pcmpAddress: pcmpAddress,
      asn1: node, // Store original node
    );
  }

  static ContactPcmpResponse_contactPcmpResponseOk decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return ContactPcmpResponse_contactPcmpResponseOk.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return ContactPcmpResponse_contactPcmpResponseOk.fromAsn1(
          node,
          tag: tag,
        );
    }
    return ContactPcmpResponse_contactPcmpResponseOk();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ContactPcmpResponse_contactPcmpResponseOk(');
    final pcmpAddressVal = pcmpAddress;
    if (pcmpAddressVal != null) {
      final val = pcmpAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  pcmpAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  pcmpAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeleteNotificationForDc {
  final NotificationMetadata? notificationMetadata;
  final EuiccSign? euiccNotificationSignature;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeleteNotificationForDc._({
    this.notificationMetadata,
    this.euiccNotificationSignature,
    this.asn1,
  });

  const DeleteNotificationForDc({
    NotificationMetadata? notificationMetadata,
    EuiccSign? euiccNotificationSignature,
  }) : this._(
         notificationMetadata: notificationMetadata,
         euiccNotificationSignature: euiccNotificationSignature,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF63;
    if (notificationMetadata != null)
      children.add((notificationMetadata as dynamic).encode(tag: 0xA0));
    if (euiccNotificationSignature != null)
      children.add(_asn1EncodeRaw(0x81, euiccNotificationSignature!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DeleteNotificationForDc.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF63;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    NotificationMetadata? notificationMetadata;
    EuiccSign? euiccNotificationSignature;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      notificationMetadata = NotificationMetadata.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      euiccNotificationSignature = child.value;
      childIdx = found + 1;
    }
    return DeleteNotificationForDc._(
      notificationMetadata: notificationMetadata,
      euiccNotificationSignature: euiccNotificationSignature,
      asn1: node, // Store original node
    );
  }

  static DeleteNotificationForDc decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return DeleteNotificationForDc.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return DeleteNotificationForDc.fromAsn1(node, tag: tag);
    }
    return DeleteNotificationForDc();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeleteNotificationForDc(');
    final notificationMetadataVal = notificationMetadata;
    if (notificationMetadataVal != null) {
      final val = notificationMetadataVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationMetadata: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  notificationMetadata: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccNotificationSignatureVal = euiccNotificationSignature;
    if (euiccNotificationSignatureVal != null) {
      final val = euiccNotificationSignatureVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccNotificationSignature: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccNotificationSignature: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeviceInfo {
  final Octet4? tac;
  final DeviceCapabilities? deviceCapabilities;
  final Octet8? imei;
  final List<Uint8List>? preferredLanguages;
  final Uint8List? deviceTestMode;
  final LpaRspCapability? lpaRspCapability;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeviceInfo._({
    this.tac,
    this.deviceCapabilities,
    this.imei,
    this.preferredLanguages,
    this.deviceTestMode,
    this.lpaRspCapability,
    this.asn1,
  });

  const DeviceInfo({
    Octet4? tac,
    DeviceCapabilities? deviceCapabilities,
    Octet8? imei,
    List<Uint8List>? preferredLanguages,
    Uint8List? deviceTestMode,
    LpaRspCapability? lpaRspCapability,
  }) : this._(
         tac: tac,
         deviceCapabilities: deviceCapabilities,
         imei: imei,
         preferredLanguages: preferredLanguages,
         deviceTestMode: deviceTestMode,
         lpaRspCapability: lpaRspCapability,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (tac != null) children.add(_asn1EncodeRaw(0x80, tac!));
    if (deviceCapabilities != null)
      children.add((deviceCapabilities as dynamic).encode(tag: 0xA1));
    if (imei != null) children.add(_asn1EncodeRaw(0x82, imei!));
    if (preferredLanguages != null) {
      final items = <Uint8List>[];
      for (final v in preferredLanguages!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xA3, items));
    }
    if (deviceTestMode != null)
      children.add(_asn1EncodeRaw(0x84, deviceTestMode!));
    if (lpaRspCapability != null)
      children.add(
        _asn1EncodeRaw(0x85, _asn1EncodeBits(lpaRspCapability!.value)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DeviceInfo.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Octet4? tac;
    DeviceCapabilities? deviceCapabilities;
    Octet8? imei;
    List<Uint8List>? preferredLanguages;
    Uint8List? deviceTestMode;
    LpaRspCapability? lpaRspCapability;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      tac = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      deviceCapabilities = DeviceCapabilities.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      imei = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA3);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Uint8List>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      preferredLanguages = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x84);
    if (found >= 0) {
      final child = node!.children[found];
      deviceTestMode = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x85);
    if (found >= 0) {
      final child = node!.children[found];
      lpaRspCapability = LpaRspCapability(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    return DeviceInfo._(
      tac: tac,
      deviceCapabilities: deviceCapabilities,
      imei: imei,
      preferredLanguages: preferredLanguages,
      deviceTestMode: deviceTestMode,
      lpaRspCapability: lpaRspCapability,
      asn1: node, // Store original node
    );
  }

  static DeviceInfo decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return DeviceInfo.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return DeviceInfo.fromAsn1(node, tag: tag);
    }
    return DeviceInfo();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeviceInfo(');
    final tacVal = tac;
    if (tacVal != null) {
      final val = tacVal;
      if (val is Uint8List) {
        buffer.writeln('  tac: ${_hex(val)},');
      } else {
        buffer.writeln('  tac: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final deviceCapabilitiesVal = deviceCapabilities;
    if (deviceCapabilitiesVal != null) {
      final val = deviceCapabilitiesVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceCapabilities: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceCapabilities: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final imeiVal = imei;
    if (imeiVal != null) {
      final val = imeiVal;
      if (val is Uint8List) {
        buffer.writeln('  imei: ${_hex(val)},');
      } else {
        buffer.writeln('  imei: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final preferredLanguagesVal = preferredLanguages;
    if (preferredLanguagesVal != null) {
      final val = preferredLanguagesVal;
      if (val is Uint8List) {
        buffer.writeln('  preferredLanguages: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  preferredLanguages: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  preferredLanguages: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deviceTestModeVal = deviceTestMode;
    if (deviceTestModeVal != null) {
      final val = deviceTestModeVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceTestMode: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceTestMode: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final lpaRspCapabilityVal = lpaRspCapability;
    if (lpaRspCapabilityVal != null) {
      final val = lpaRspCapabilityVal;
      if (val is Uint8List) {
        buffer.writeln('  lpaRspCapability: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  lpaRspCapability: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeviceCapabilities {
  final VersionType? gsmSupportedRelease;
  final VersionType? utranSupportedRelease;
  final VersionType? cdma2000onexSupportedRelease;
  final VersionType? cdma2000hrpdSupportedRelease;
  final VersionType? cdma2000ehrpdSupportedRelease;
  final VersionType? eutranEpcSupportedRelease;
  final VersionType? contactlessSupportedRelease;
  final VersionType? rspCrlSupportedVersion;
  final VersionType? nrEpcSupportedRelease;
  final VersionType? nr5gcSupportedRelease;
  final VersionType? eutran5gcSupportedRelease;
  final VersionType? lpaSvn;
  final CatSupportedClasses? catSupportedClasses;
  final EuiccFormFactorType? euiccFormFactorType;
  final DeviceAdditionalFeatureSupport? deviceAdditionalFeatureSupport;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeviceCapabilities._({
    this.gsmSupportedRelease,
    this.utranSupportedRelease,
    this.cdma2000onexSupportedRelease,
    this.cdma2000hrpdSupportedRelease,
    this.cdma2000ehrpdSupportedRelease,
    this.eutranEpcSupportedRelease,
    this.contactlessSupportedRelease,
    this.rspCrlSupportedVersion,
    this.nrEpcSupportedRelease,
    this.nr5gcSupportedRelease,
    this.eutran5gcSupportedRelease,
    this.lpaSvn,
    this.catSupportedClasses,
    this.euiccFormFactorType,
    this.deviceAdditionalFeatureSupport,
    this.asn1,
  });

  const DeviceCapabilities({
    VersionType? gsmSupportedRelease,
    VersionType? utranSupportedRelease,
    VersionType? cdma2000onexSupportedRelease,
    VersionType? cdma2000hrpdSupportedRelease,
    VersionType? cdma2000ehrpdSupportedRelease,
    VersionType? eutranEpcSupportedRelease,
    VersionType? contactlessSupportedRelease,
    VersionType? rspCrlSupportedVersion,
    VersionType? nrEpcSupportedRelease,
    VersionType? nr5gcSupportedRelease,
    VersionType? eutran5gcSupportedRelease,
    VersionType? lpaSvn,
    CatSupportedClasses? catSupportedClasses,
    EuiccFormFactorType? euiccFormFactorType,
    DeviceAdditionalFeatureSupport? deviceAdditionalFeatureSupport,
  }) : this._(
         gsmSupportedRelease: gsmSupportedRelease,
         utranSupportedRelease: utranSupportedRelease,
         cdma2000onexSupportedRelease: cdma2000onexSupportedRelease,
         cdma2000hrpdSupportedRelease: cdma2000hrpdSupportedRelease,
         cdma2000ehrpdSupportedRelease: cdma2000ehrpdSupportedRelease,
         eutranEpcSupportedRelease: eutranEpcSupportedRelease,
         contactlessSupportedRelease: contactlessSupportedRelease,
         rspCrlSupportedVersion: rspCrlSupportedVersion,
         nrEpcSupportedRelease: nrEpcSupportedRelease,
         nr5gcSupportedRelease: nr5gcSupportedRelease,
         eutran5gcSupportedRelease: eutran5gcSupportedRelease,
         lpaSvn: lpaSvn,
         catSupportedClasses: catSupportedClasses,
         euiccFormFactorType: euiccFormFactorType,
         deviceAdditionalFeatureSupport: deviceAdditionalFeatureSupport,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (gsmSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x80, gsmSupportedRelease!));
    if (utranSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x81, utranSupportedRelease!));
    if (cdma2000onexSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x82, cdma2000onexSupportedRelease!));
    if (cdma2000hrpdSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x83, cdma2000hrpdSupportedRelease!));
    if (cdma2000ehrpdSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x84, cdma2000ehrpdSupportedRelease!));
    if (eutranEpcSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x85, eutranEpcSupportedRelease!));
    if (contactlessSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x86, contactlessSupportedRelease!));
    if (rspCrlSupportedVersion != null)
      children.add(_asn1EncodeRaw(0x87, rspCrlSupportedVersion!));
    if (nrEpcSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x88, nrEpcSupportedRelease!));
    if (nr5gcSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x89, nr5gcSupportedRelease!));
    if (eutran5gcSupportedRelease != null)
      children.add(_asn1EncodeRaw(0x8A, eutran5gcSupportedRelease!));
    if (lpaSvn != null) children.add(_asn1EncodeRaw(0x8B, lpaSvn!));
    if (catSupportedClasses != null)
      children.add(
        _asn1EncodeRaw(0x8C, _asn1EncodeBits(catSupportedClasses!.value)),
      );
    if (euiccFormFactorType != null)
      children.add(
        _asn1EncodeRaw(0x8D, _asn1EncodeInt(euiccFormFactorType!.value)),
      );
    if (deviceAdditionalFeatureSupport != null)
      children.add(
        (deviceAdditionalFeatureSupport as dynamic).encode(tag: 0xAE),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DeviceCapabilities.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    VersionType? gsmSupportedRelease;
    VersionType? utranSupportedRelease;
    VersionType? cdma2000onexSupportedRelease;
    VersionType? cdma2000hrpdSupportedRelease;
    VersionType? cdma2000ehrpdSupportedRelease;
    VersionType? eutranEpcSupportedRelease;
    VersionType? contactlessSupportedRelease;
    VersionType? rspCrlSupportedVersion;
    VersionType? nrEpcSupportedRelease;
    VersionType? nr5gcSupportedRelease;
    VersionType? eutran5gcSupportedRelease;
    VersionType? lpaSvn;
    CatSupportedClasses? catSupportedClasses;
    EuiccFormFactorType? euiccFormFactorType;
    DeviceAdditionalFeatureSupport? deviceAdditionalFeatureSupport;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      gsmSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      utranSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      cdma2000onexSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      cdma2000hrpdSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x84);
    if (found >= 0) {
      final child = node!.children[found];
      cdma2000ehrpdSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x85);
    if (found >= 0) {
      final child = node!.children[found];
      eutranEpcSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x86);
    if (found >= 0) {
      final child = node!.children[found];
      contactlessSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x87);
    if (found >= 0) {
      final child = node!.children[found];
      rspCrlSupportedVersion = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x88);
    if (found >= 0) {
      final child = node!.children[found];
      nrEpcSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x89);
    if (found >= 0) {
      final child = node!.children[found];
      nr5gcSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x8A);
    if (found >= 0) {
      final child = node!.children[found];
      eutran5gcSupportedRelease = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x8B);
    if (found >= 0) {
      final child = node!.children[found];
      lpaSvn = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x8C);
    if (found >= 0) {
      final child = node!.children[found];
      catSupportedClasses = CatSupportedClasses(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x8D);
    if (found >= 0) {
      final child = node!.children[found];
      euiccFormFactorType = EuiccFormFactorTypeExt.fromValue(
        _asn1DecodeInt(child.value),
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xAE);
    if (found >= 0) {
      final child = node!.children[found];
      deviceAdditionalFeatureSupport = DeviceAdditionalFeatureSupport.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    return DeviceCapabilities._(
      gsmSupportedRelease: gsmSupportedRelease,
      utranSupportedRelease: utranSupportedRelease,
      cdma2000onexSupportedRelease: cdma2000onexSupportedRelease,
      cdma2000hrpdSupportedRelease: cdma2000hrpdSupportedRelease,
      cdma2000ehrpdSupportedRelease: cdma2000ehrpdSupportedRelease,
      eutranEpcSupportedRelease: eutranEpcSupportedRelease,
      contactlessSupportedRelease: contactlessSupportedRelease,
      rspCrlSupportedVersion: rspCrlSupportedVersion,
      nrEpcSupportedRelease: nrEpcSupportedRelease,
      nr5gcSupportedRelease: nr5gcSupportedRelease,
      eutran5gcSupportedRelease: eutran5gcSupportedRelease,
      lpaSvn: lpaSvn,
      catSupportedClasses: catSupportedClasses,
      euiccFormFactorType: euiccFormFactorType,
      deviceAdditionalFeatureSupport: deviceAdditionalFeatureSupport,
      asn1: node, // Store original node
    );
  }

  static DeviceCapabilities decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return DeviceCapabilities.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return DeviceCapabilities.fromAsn1(node, tag: tag);
    }
    return DeviceCapabilities();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeviceCapabilities(');
    final gsmSupportedReleaseVal = gsmSupportedRelease;
    if (gsmSupportedReleaseVal != null) {
      final val = gsmSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  gsmSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  gsmSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final utranSupportedReleaseVal = utranSupportedRelease;
    if (utranSupportedReleaseVal != null) {
      final val = utranSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  utranSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  utranSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final cdma2000onexSupportedReleaseVal = cdma2000onexSupportedRelease;
    if (cdma2000onexSupportedReleaseVal != null) {
      final val = cdma2000onexSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  cdma2000onexSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  cdma2000onexSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final cdma2000hrpdSupportedReleaseVal = cdma2000hrpdSupportedRelease;
    if (cdma2000hrpdSupportedReleaseVal != null) {
      final val = cdma2000hrpdSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  cdma2000hrpdSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  cdma2000hrpdSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final cdma2000ehrpdSupportedReleaseVal = cdma2000ehrpdSupportedRelease;
    if (cdma2000ehrpdSupportedReleaseVal != null) {
      final val = cdma2000ehrpdSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  cdma2000ehrpdSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  cdma2000ehrpdSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final eutranEpcSupportedReleaseVal = eutranEpcSupportedRelease;
    if (eutranEpcSupportedReleaseVal != null) {
      final val = eutranEpcSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  eutranEpcSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  eutranEpcSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final contactlessSupportedReleaseVal = contactlessSupportedRelease;
    if (contactlessSupportedReleaseVal != null) {
      final val = contactlessSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  contactlessSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  contactlessSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rspCrlSupportedVersionVal = rspCrlSupportedVersion;
    if (rspCrlSupportedVersionVal != null) {
      final val = rspCrlSupportedVersionVal;
      if (val is Uint8List) {
        buffer.writeln('  rspCrlSupportedVersion: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rspCrlSupportedVersion: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final nrEpcSupportedReleaseVal = nrEpcSupportedRelease;
    if (nrEpcSupportedReleaseVal != null) {
      final val = nrEpcSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  nrEpcSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  nrEpcSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final nr5gcSupportedReleaseVal = nr5gcSupportedRelease;
    if (nr5gcSupportedReleaseVal != null) {
      final val = nr5gcSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  nr5gcSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  nr5gcSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final eutran5gcSupportedReleaseVal = eutran5gcSupportedRelease;
    if (eutran5gcSupportedReleaseVal != null) {
      final val = eutran5gcSupportedReleaseVal;
      if (val is Uint8List) {
        buffer.writeln('  eutran5gcSupportedRelease: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  eutran5gcSupportedRelease: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final lpaSvnVal = lpaSvn;
    if (lpaSvnVal != null) {
      final val = lpaSvnVal;
      if (val is Uint8List) {
        buffer.writeln('  lpaSvn: ${_hex(val)},');
      } else {
        buffer.writeln('  lpaSvn: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final catSupportedClassesVal = catSupportedClasses;
    if (catSupportedClassesVal != null) {
      final val = catSupportedClassesVal;
      if (val is Uint8List) {
        buffer.writeln('  catSupportedClasses: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  catSupportedClasses: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccFormFactorTypeVal = euiccFormFactorType;
    if (euiccFormFactorTypeVal != null) {
      final val = euiccFormFactorTypeVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccFormFactorType: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccFormFactorType: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deviceAdditionalFeatureSupportVal = deviceAdditionalFeatureSupport;
    if (deviceAdditionalFeatureSupportVal != null) {
      final val = deviceAdditionalFeatureSupportVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceAdditionalFeatureSupport: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceAdditionalFeatureSupport: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeviceAdditionalFeatureSupport {
  final VersionType? naiSupport;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeviceAdditionalFeatureSupport._({this.naiSupport, this.asn1});

  const DeviceAdditionalFeatureSupport({VersionType? naiSupport})
    : this._(naiSupport: naiSupport, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (naiSupport != null) children.add(_asn1EncodeRaw(0x80, naiSupport!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DeviceAdditionalFeatureSupport.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    VersionType? naiSupport;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      naiSupport = child.value;
      childIdx = found + 1;
    }
    return DeviceAdditionalFeatureSupport._(
      naiSupport: naiSupport,
      asn1: node, // Store original node
    );
  }

  static DeviceAdditionalFeatureSupport decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return DeviceAdditionalFeatureSupport.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return DeviceAdditionalFeatureSupport.fromAsn1(node, tag: tag);
    }
    return DeviceAdditionalFeatureSupport();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeviceAdditionalFeatureSupport(');
    final naiSupportVal = naiSupport;
    if (naiSupportVal != null) {
      final val = naiSupportVal;
      if (val is Uint8List) {
        buffer.writeln('  naiSupport: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  naiSupport: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EUICCInfo1 {
  final VersionType? lowestSvn;
  final List<SubjectKeyIdentifier>? euiccCiPKIdListForVerification;
  final List<SubjectKeyIdentifier>? euiccCiPKIdListForSigning;
  final List<SubjectKeyIdentifier>? euiccCiPKIdListForSigningV3;
  final EuiccRspCapability? euiccRspCapability;
  final VersionType? highestSvn;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EUICCInfo1._({
    this.lowestSvn,
    this.euiccCiPKIdListForVerification,
    this.euiccCiPKIdListForSigning,
    this.euiccCiPKIdListForSigningV3,
    this.euiccRspCapability,
    this.highestSvn,
    this.asn1,
  });

  const EUICCInfo1({
    VersionType? lowestSvn,
    List<SubjectKeyIdentifier>? euiccCiPKIdListForVerification,
    List<SubjectKeyIdentifier>? euiccCiPKIdListForSigning,
    List<SubjectKeyIdentifier>? euiccCiPKIdListForSigningV3,
    EuiccRspCapability? euiccRspCapability,
    VersionType? highestSvn,
  }) : this._(
         lowestSvn: lowestSvn,
         euiccCiPKIdListForVerification: euiccCiPKIdListForVerification,
         euiccCiPKIdListForSigning: euiccCiPKIdListForSigning,
         euiccCiPKIdListForSigningV3: euiccCiPKIdListForSigningV3,
         euiccRspCapability: euiccRspCapability,
         highestSvn: highestSvn,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF20;
    if (lowestSvn != null) children.add(_asn1EncodeRaw(0x82, lowestSvn!));
    if (euiccCiPKIdListForVerification != null) {
      final items = <Uint8List>[];
      for (final v in euiccCiPKIdListForVerification!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xA9, items));
    }
    if (euiccCiPKIdListForSigning != null) {
      final items = <Uint8List>[];
      for (final v in euiccCiPKIdListForSigning!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xAA, items));
    }
    if (euiccCiPKIdListForSigningV3 != null) {
      final items = <Uint8List>[];
      for (final v in euiccCiPKIdListForSigningV3!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xB1, items));
    }
    if (euiccRspCapability != null)
      children.add(
        _asn1EncodeRaw(0x88, _asn1EncodeBits(euiccRspCapability!.value)),
      );
    if (highestSvn != null) children.add(_asn1EncodeRaw(0x93, highestSvn!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EUICCInfo1.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF20;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    VersionType? lowestSvn;
    List<SubjectKeyIdentifier>? euiccCiPKIdListForVerification;
    List<SubjectKeyIdentifier>? euiccCiPKIdListForSigning;
    List<SubjectKeyIdentifier>? euiccCiPKIdListForSigningV3;
    EuiccRspCapability? euiccRspCapability;
    VersionType? highestSvn;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      lowestSvn = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA9);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <SubjectKeyIdentifier>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      euiccCiPKIdListForVerification = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xAA);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <SubjectKeyIdentifier>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      euiccCiPKIdListForSigning = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB1);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <SubjectKeyIdentifier>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      euiccCiPKIdListForSigningV3 = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x88);
    if (found >= 0) {
      final child = node!.children[found];
      euiccRspCapability = EuiccRspCapability(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x93);
    if (found >= 0) {
      final child = node!.children[found];
      highestSvn = child.value;
      childIdx = found + 1;
    }
    return EUICCInfo1._(
      lowestSvn: lowestSvn,
      euiccCiPKIdListForVerification: euiccCiPKIdListForVerification,
      euiccCiPKIdListForSigning: euiccCiPKIdListForSigning,
      euiccCiPKIdListForSigningV3: euiccCiPKIdListForSigningV3,
      euiccRspCapability: euiccRspCapability,
      highestSvn: highestSvn,
      asn1: node, // Store original node
    );
  }

  static EUICCInfo1 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return EUICCInfo1.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return EUICCInfo1.fromAsn1(node, tag: tag);
    }
    return EUICCInfo1();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EUICCInfo1(');
    final lowestSvnVal = lowestSvn;
    if (lowestSvnVal != null) {
      final val = lowestSvnVal;
      if (val is Uint8List) {
        buffer.writeln('  lowestSvn: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  lowestSvn: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdListForVerificationVal = euiccCiPKIdListForVerification;
    if (euiccCiPKIdListForVerificationVal != null) {
      final val = euiccCiPKIdListForVerificationVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdListForVerification: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  euiccCiPKIdListForVerification: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  euiccCiPKIdListForVerification: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdListForSigningVal = euiccCiPKIdListForSigning;
    if (euiccCiPKIdListForSigningVal != null) {
      final val = euiccCiPKIdListForSigningVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdListForSigning: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  euiccCiPKIdListForSigning: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  euiccCiPKIdListForSigning: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdListForSigningV3Val = euiccCiPKIdListForSigningV3;
    if (euiccCiPKIdListForSigningV3Val != null) {
      final val = euiccCiPKIdListForSigningV3Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdListForSigningV3: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  euiccCiPKIdListForSigningV3: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  euiccCiPKIdListForSigningV3: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccRspCapabilityVal = euiccRspCapability;
    if (euiccRspCapabilityVal != null) {
      final val = euiccRspCapabilityVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccRspCapability: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccRspCapability: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final highestSvnVal = highestSvn;
    if (highestSvnVal != null) {
      final val = highestSvnVal;
      if (val is Uint8List) {
        buffer.writeln('  highestSvn: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  highestSvn: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EUICCInfo2 {
  final VersionType? baseProfilePackageVersion;
  final VersionType? lowestSvn;
  final VersionType? euiccFirmwareVersion;
  final ExtendedCardResource? extCardResource;
  final UICCCapability? uiccCapability;
  final VersionType? ts102241Version;
  final VersionType? globalplatformVersion;
  final EuiccRspCapability? euiccRspCapability;
  final List<SubjectKeyIdentifier>? euiccCiPKIdListForVerification;
  final List<SubjectKeyIdentifier>? euiccCiPKIdListForSigning;
  final EUICCInfo2_euiccCategory? euiccCategory;
  final PprIds? forbiddenProfilePolicyRules;
  final VersionType? ppVersion;
  final String? sasAcreditationNumber;
  final CertificationDataObject? certificationDataObject;
  final int? treProperties;
  final String? treProductReference;
  final List<VersionType>? additionalProfilePackageVersions;
  final LpaMode? lpaMode;
  final List<SubjectKeyIdentifier>? euiccCiPKIdListForSigningV3;
  final Uint8List? additionalEuiccInfo;
  final VersionType? highestSvn;
  final IoTSpecificInfo? iotSpecificInfo;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EUICCInfo2._({
    this.baseProfilePackageVersion,
    this.lowestSvn,
    this.euiccFirmwareVersion,
    this.extCardResource,
    this.uiccCapability,
    this.ts102241Version,
    this.globalplatformVersion,
    this.euiccRspCapability,
    this.euiccCiPKIdListForVerification,
    this.euiccCiPKIdListForSigning,
    this.euiccCategory,
    this.forbiddenProfilePolicyRules,
    this.ppVersion,
    this.sasAcreditationNumber,
    this.certificationDataObject,
    this.treProperties,
    this.treProductReference,
    this.additionalProfilePackageVersions,
    this.lpaMode,
    this.euiccCiPKIdListForSigningV3,
    this.additionalEuiccInfo,
    this.highestSvn,
    this.iotSpecificInfo,
    this.asn1,
  });

  const EUICCInfo2({
    VersionType? baseProfilePackageVersion,
    VersionType? lowestSvn,
    VersionType? euiccFirmwareVersion,
    ExtendedCardResource? extCardResource,
    UICCCapability? uiccCapability,
    VersionType? ts102241Version,
    VersionType? globalplatformVersion,
    EuiccRspCapability? euiccRspCapability,
    List<SubjectKeyIdentifier>? euiccCiPKIdListForVerification,
    List<SubjectKeyIdentifier>? euiccCiPKIdListForSigning,
    EUICCInfo2_euiccCategory? euiccCategory,
    PprIds? forbiddenProfilePolicyRules,
    VersionType? ppVersion,
    String? sasAcreditationNumber,
    CertificationDataObject? certificationDataObject,
    int? treProperties,
    String? treProductReference,
    List<VersionType>? additionalProfilePackageVersions,
    LpaMode? lpaMode,
    List<SubjectKeyIdentifier>? euiccCiPKIdListForSigningV3,
    Uint8List? additionalEuiccInfo,
    VersionType? highestSvn,
    IoTSpecificInfo? iotSpecificInfo,
  }) : this._(
         baseProfilePackageVersion: baseProfilePackageVersion,
         lowestSvn: lowestSvn,
         euiccFirmwareVersion: euiccFirmwareVersion,
         extCardResource: extCardResource,
         uiccCapability: uiccCapability,
         ts102241Version: ts102241Version,
         globalplatformVersion: globalplatformVersion,
         euiccRspCapability: euiccRspCapability,
         euiccCiPKIdListForVerification: euiccCiPKIdListForVerification,
         euiccCiPKIdListForSigning: euiccCiPKIdListForSigning,
         euiccCategory: euiccCategory,
         forbiddenProfilePolicyRules: forbiddenProfilePolicyRules,
         ppVersion: ppVersion,
         sasAcreditationNumber: sasAcreditationNumber,
         certificationDataObject: certificationDataObject,
         treProperties: treProperties,
         treProductReference: treProductReference,
         additionalProfilePackageVersions: additionalProfilePackageVersions,
         lpaMode: lpaMode,
         euiccCiPKIdListForSigningV3: euiccCiPKIdListForSigningV3,
         additionalEuiccInfo: additionalEuiccInfo,
         highestSvn: highestSvn,
         iotSpecificInfo: iotSpecificInfo,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF22;
    if (baseProfilePackageVersion != null)
      children.add(_asn1EncodeRaw(0x81, baseProfilePackageVersion!));
    if (lowestSvn != null) children.add(_asn1EncodeRaw(0x82, lowestSvn!));
    if (euiccFirmwareVersion != null)
      children.add(_asn1EncodeRaw(0x83, euiccFirmwareVersion!));
    if (extCardResource != null)
      children.add((extCardResource as dynamic).encode(tag: 0xA4));
    if (uiccCapability != null)
      children.add(
        _asn1EncodeRaw(0x85, _asn1EncodeBits(uiccCapability!.value)),
      );
    if (ts102241Version != null)
      children.add(_asn1EncodeRaw(0x86, ts102241Version!));
    if (globalplatformVersion != null)
      children.add(_asn1EncodeRaw(0x87, globalplatformVersion!));
    if (euiccRspCapability != null)
      children.add(
        _asn1EncodeRaw(0x88, _asn1EncodeBits(euiccRspCapability!.value)),
      );
    if (euiccCiPKIdListForVerification != null) {
      final items = <Uint8List>[];
      for (final v in euiccCiPKIdListForVerification!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xA9, items));
    }
    if (euiccCiPKIdListForSigning != null) {
      final items = <Uint8List>[];
      for (final v in euiccCiPKIdListForSigning!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xAA, items));
    }
    if (euiccCategory != null)
      children.add(_asn1EncodeRaw(0x8B, _asn1EncodeInt(euiccCategory!.value)));
    if (forbiddenProfilePolicyRules != null)
      children.add(
        _asn1EncodeRaw(
          0x99,
          _asn1EncodeBits(forbiddenProfilePolicyRules!.value),
        ),
      );
    if (ppVersion != null) children.add(_asn1EncodeRaw(0x4, ppVersion!));
    if (sasAcreditationNumber != null)
      children.add(
        _asn1EncodeRaw(
          0xC,
          Uint8List.fromList(sasAcreditationNumber!.codeUnits),
        ),
      );
    if (certificationDataObject != null)
      children.add((certificationDataObject as dynamic).encode(tag: 0xAC));
    if (treProperties != null)
      children.add(_asn1EncodeRaw(0x8D, _asn1EncodeInt(treProperties!)));
    if (treProductReference != null)
      children.add(
        _asn1EncodeRaw(
          0x8E,
          Uint8List.fromList(treProductReference!.codeUnits),
        ),
      );
    if (additionalProfilePackageVersions != null) {
      final items = <Uint8List>[];
      for (final v in additionalProfilePackageVersions!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xAF, items));
    }
    if (lpaMode != null)
      children.add(_asn1EncodeRaw(0x90, _asn1EncodeInt(lpaMode!.value)));
    if (euiccCiPKIdListForSigningV3 != null) {
      final items = <Uint8List>[];
      for (final v in euiccCiPKIdListForSigningV3!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xB1, items));
    }
    if (additionalEuiccInfo != null)
      children.add(_asn1EncodeRaw(0x92, additionalEuiccInfo!));
    if (highestSvn != null) children.add(_asn1EncodeRaw(0x93, highestSvn!));
    if (iotSpecificInfo != null)
      children.add((iotSpecificInfo as dynamic).encode(tag: 0xB4));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EUICCInfo2.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF22;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    VersionType? baseProfilePackageVersion;
    VersionType? lowestSvn;
    VersionType? euiccFirmwareVersion;
    ExtendedCardResource? extCardResource;
    UICCCapability? uiccCapability;
    VersionType? ts102241Version;
    VersionType? globalplatformVersion;
    EuiccRspCapability? euiccRspCapability;
    List<SubjectKeyIdentifier>? euiccCiPKIdListForVerification;
    List<SubjectKeyIdentifier>? euiccCiPKIdListForSigning;
    EUICCInfo2_euiccCategory? euiccCategory;
    PprIds? forbiddenProfilePolicyRules;
    VersionType? ppVersion;
    String? sasAcreditationNumber;
    CertificationDataObject? certificationDataObject;
    int? treProperties;
    String? treProductReference;
    List<VersionType>? additionalProfilePackageVersions;
    LpaMode? lpaMode;
    List<SubjectKeyIdentifier>? euiccCiPKIdListForSigningV3;
    Uint8List? additionalEuiccInfo;
    VersionType? highestSvn;
    IoTSpecificInfo? iotSpecificInfo;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      baseProfilePackageVersion = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      lowestSvn = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      euiccFirmwareVersion = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA4);
    if (found >= 0) {
      final child = node!.children[found];
      extCardResource = ExtendedCardResource.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x85);
    if (found >= 0) {
      final child = node!.children[found];
      uiccCapability = UICCCapability(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x86);
    if (found >= 0) {
      final child = node!.children[found];
      ts102241Version = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x87);
    if (found >= 0) {
      final child = node!.children[found];
      globalplatformVersion = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x88);
    if (found >= 0) {
      final child = node!.children[found];
      euiccRspCapability = EuiccRspCapability(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA9);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <SubjectKeyIdentifier>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      euiccCiPKIdListForVerification = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xAA);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <SubjectKeyIdentifier>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      euiccCiPKIdListForSigning = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x8B);
    if (found >= 0) {
      final child = node!.children[found];
      euiccCategory = EUICCInfo2_euiccCategoryExt.fromValue(
        _asn1DecodeInt(child.value),
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x99);
    if (found >= 0) {
      final child = node!.children[found];
      forbiddenProfilePolicyRules = PprIds(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      ppVersion = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xC);
    if (found >= 0) {
      final child = node!.children[found];
      sasAcreditationNumber = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xAC);
    if (found >= 0) {
      final child = node!.children[found];
      certificationDataObject = CertificationDataObject.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x8D);
    if (found >= 0) {
      final child = node!.children[found];
      treProperties = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x8E);
    if (found >= 0) {
      final child = node!.children[found];
      treProductReference = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xAF);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <VersionType>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      additionalProfilePackageVersions = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x90);
    if (found >= 0) {
      final child = node!.children[found];
      lpaMode = LpaModeExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB1);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <SubjectKeyIdentifier>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      euiccCiPKIdListForSigningV3 = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x92);
    if (found >= 0) {
      final child = node!.children[found];
      additionalEuiccInfo = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x93);
    if (found >= 0) {
      final child = node!.children[found];
      highestSvn = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB4);
    if (found >= 0) {
      final child = node!.children[found];
      iotSpecificInfo = IoTSpecificInfo.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    return EUICCInfo2._(
      baseProfilePackageVersion: baseProfilePackageVersion,
      lowestSvn: lowestSvn,
      euiccFirmwareVersion: euiccFirmwareVersion,
      extCardResource: extCardResource,
      uiccCapability: uiccCapability,
      ts102241Version: ts102241Version,
      globalplatformVersion: globalplatformVersion,
      euiccRspCapability: euiccRspCapability,
      euiccCiPKIdListForVerification: euiccCiPKIdListForVerification,
      euiccCiPKIdListForSigning: euiccCiPKIdListForSigning,
      euiccCategory: euiccCategory,
      forbiddenProfilePolicyRules: forbiddenProfilePolicyRules,
      ppVersion: ppVersion,
      sasAcreditationNumber: sasAcreditationNumber,
      certificationDataObject: certificationDataObject,
      treProperties: treProperties,
      treProductReference: treProductReference,
      additionalProfilePackageVersions: additionalProfilePackageVersions,
      lpaMode: lpaMode,
      euiccCiPKIdListForSigningV3: euiccCiPKIdListForSigningV3,
      additionalEuiccInfo: additionalEuiccInfo,
      highestSvn: highestSvn,
      iotSpecificInfo: iotSpecificInfo,
      asn1: node, // Store original node
    );
  }

  static EUICCInfo2 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return EUICCInfo2.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return EUICCInfo2.fromAsn1(node, tag: tag);
    }
    return EUICCInfo2();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EUICCInfo2(');
    final baseProfilePackageVersionVal = baseProfilePackageVersion;
    if (baseProfilePackageVersionVal != null) {
      final val = baseProfilePackageVersionVal;
      if (val is Uint8List) {
        buffer.writeln('  baseProfilePackageVersion: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  baseProfilePackageVersion: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final lowestSvnVal = lowestSvn;
    if (lowestSvnVal != null) {
      final val = lowestSvnVal;
      if (val is Uint8List) {
        buffer.writeln('  lowestSvn: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  lowestSvn: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccFirmwareVersionVal = euiccFirmwareVersion;
    if (euiccFirmwareVersionVal != null) {
      final val = euiccFirmwareVersionVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccFirmwareVersion: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccFirmwareVersion: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final extCardResourceVal = extCardResource;
    if (extCardResourceVal != null) {
      final val = extCardResourceVal;
      if (val is Uint8List) {
        buffer.writeln('  extCardResource: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  extCardResource: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final uiccCapabilityVal = uiccCapability;
    if (uiccCapabilityVal != null) {
      final val = uiccCapabilityVal;
      if (val is Uint8List) {
        buffer.writeln('  uiccCapability: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  uiccCapability: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ts102241VersionVal = ts102241Version;
    if (ts102241VersionVal != null) {
      final val = ts102241VersionVal;
      if (val is Uint8List) {
        buffer.writeln('  ts102241Version: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ts102241Version: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final globalplatformVersionVal = globalplatformVersion;
    if (globalplatformVersionVal != null) {
      final val = globalplatformVersionVal;
      if (val is Uint8List) {
        buffer.writeln('  globalplatformVersion: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  globalplatformVersion: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccRspCapabilityVal = euiccRspCapability;
    if (euiccRspCapabilityVal != null) {
      final val = euiccRspCapabilityVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccRspCapability: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccRspCapability: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdListForVerificationVal = euiccCiPKIdListForVerification;
    if (euiccCiPKIdListForVerificationVal != null) {
      final val = euiccCiPKIdListForVerificationVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdListForVerification: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  euiccCiPKIdListForVerification: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  euiccCiPKIdListForVerification: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdListForSigningVal = euiccCiPKIdListForSigning;
    if (euiccCiPKIdListForSigningVal != null) {
      final val = euiccCiPKIdListForSigningVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdListForSigning: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  euiccCiPKIdListForSigning: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  euiccCiPKIdListForSigning: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCategoryVal = euiccCategory;
    if (euiccCategoryVal != null) {
      final val = euiccCategoryVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCategory: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccCategory: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final forbiddenProfilePolicyRulesVal = forbiddenProfilePolicyRules;
    if (forbiddenProfilePolicyRulesVal != null) {
      final val = forbiddenProfilePolicyRulesVal;
      if (val is Uint8List) {
        buffer.writeln('  forbiddenProfilePolicyRules: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  forbiddenProfilePolicyRules: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ppVersionVal = ppVersion;
    if (ppVersionVal != null) {
      final val = ppVersionVal;
      if (val is Uint8List) {
        buffer.writeln('  ppVersion: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ppVersion: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final sasAcreditationNumberVal = sasAcreditationNumber;
    if (sasAcreditationNumberVal != null) {
      final val = sasAcreditationNumberVal;
      if (val is Uint8List) {
        buffer.writeln('  sasAcreditationNumber: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  sasAcreditationNumber: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final certificationDataObjectVal = certificationDataObject;
    if (certificationDataObjectVal != null) {
      final val = certificationDataObjectVal;
      if (val is Uint8List) {
        buffer.writeln('  certificationDataObject: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  certificationDataObject: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final trePropertiesVal = treProperties;
    if (trePropertiesVal != null) {
      final val = trePropertiesVal;
      if (val is Uint8List) {
        buffer.writeln('  treProperties: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  treProperties: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final treProductReferenceVal = treProductReference;
    if (treProductReferenceVal != null) {
      final val = treProductReferenceVal;
      if (val is Uint8List) {
        buffer.writeln('  treProductReference: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  treProductReference: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final additionalProfilePackageVersionsVal =
        additionalProfilePackageVersions;
    if (additionalProfilePackageVersionsVal != null) {
      final val = additionalProfilePackageVersionsVal;
      if (val is Uint8List) {
        buffer.writeln('  additionalProfilePackageVersions: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  additionalProfilePackageVersions: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  additionalProfilePackageVersions: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final lpaModeVal = lpaMode;
    if (lpaModeVal != null) {
      final val = lpaModeVal;
      if (val is Uint8List) {
        buffer.writeln('  lpaMode: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  lpaMode: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdListForSigningV3Val = euiccCiPKIdListForSigningV3;
    if (euiccCiPKIdListForSigningV3Val != null) {
      final val = euiccCiPKIdListForSigningV3Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdListForSigningV3: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  euiccCiPKIdListForSigningV3: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  euiccCiPKIdListForSigningV3: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final additionalEuiccInfoVal = additionalEuiccInfo;
    if (additionalEuiccInfoVal != null) {
      final val = additionalEuiccInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  additionalEuiccInfo: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  additionalEuiccInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final highestSvnVal = highestSvn;
    if (highestSvnVal != null) {
      final val = highestSvnVal;
      if (val is Uint8List) {
        buffer.writeln('  highestSvn: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  highestSvn: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iotSpecificInfoVal = iotSpecificInfo;
    if (iotSpecificInfoVal != null) {
      final val = iotSpecificInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  iotSpecificInfo: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  iotSpecificInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CertificationDataObject {
  final String? platformLabel;
  final String? discoveryBaseURL;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CertificationDataObject._({
    this.platformLabel,
    this.discoveryBaseURL,
    this.asn1,
  });

  const CertificationDataObject({
    String? platformLabel,
    String? discoveryBaseURL,
  }) : this._(
         platformLabel: platformLabel,
         discoveryBaseURL: discoveryBaseURL,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (platformLabel != null)
      children.add(
        _asn1EncodeRaw(0x80, Uint8List.fromList(platformLabel!.codeUnits)),
      );
    if (discoveryBaseURL != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList(discoveryBaseURL!.codeUnits)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CertificationDataObject.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    String? platformLabel;
    String? discoveryBaseURL;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      platformLabel = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      discoveryBaseURL = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return CertificationDataObject._(
      platformLabel: platformLabel,
      discoveryBaseURL: discoveryBaseURL,
      asn1: node, // Store original node
    );
  }

  static CertificationDataObject decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return CertificationDataObject.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return CertificationDataObject.fromAsn1(node, tag: tag);
    }
    return CertificationDataObject();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CertificationDataObject(');
    final platformLabelVal = platformLabel;
    if (platformLabelVal != null) {
      final val = platformLabelVal;
      if (val is Uint8List) {
        buffer.writeln('  platformLabel: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  platformLabel: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final discoveryBaseURLVal = discoveryBaseURL;
    if (discoveryBaseURLVal != null) {
      final val = discoveryBaseURLVal;
      if (val is Uint8List) {
        buffer.writeln('  discoveryBaseURL: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  discoveryBaseURL: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class IoTSpecificInfo {
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const IoTSpecificInfo._({this.asn1});
  const IoTSpecificInfo() : this._(asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory IoTSpecificInfo.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    int childIdx = 0;
    return IoTSpecificInfo._(
      asn1: node, // Store original node
    );
  }

  static IoTSpecificInfo decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return IoTSpecificInfo.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return IoTSpecificInfo.fromAsn1(node, tag: tag);
    }
    return IoTSpecificInfo();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('IoTSpecificInfo(');
    buffer.write(')');
    return buffer.toString();
  }
}

class UpdateMetadataRequest {
  final String? serviceProviderName;
  final String? profileName;
  final IconType? iconType;
  final Uint8List? icon;
  final PprIds? profilePolicyRules;
  final VendorSpecificExtension? serviceSpecificDataStoredInEuicc;
  final List<NotificationConfigurationInformation>?
  notificationConfigurationInfo;
  final Uint8List? tagsForDeletion;
  final RpmConfiguration? rpmConfiguration;
  final String? hriServerAddress;
  final LprConfiguration? lprConfiguration;
  final EnterpriseConfiguration? enterpriseConfiguration;
  final DeviceChangeConfiguration? deviceChangeConfiguration;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const UpdateMetadataRequest._({
    this.serviceProviderName,
    this.profileName,
    this.iconType,
    this.icon,
    this.profilePolicyRules,
    this.serviceSpecificDataStoredInEuicc,
    this.notificationConfigurationInfo,
    this.tagsForDeletion,
    this.rpmConfiguration,
    this.hriServerAddress,
    this.lprConfiguration,
    this.enterpriseConfiguration,
    this.deviceChangeConfiguration,
    this.asn1,
  });

  const UpdateMetadataRequest({
    String? serviceProviderName,
    String? profileName,
    IconType? iconType,
    Uint8List? icon,
    PprIds? profilePolicyRules,
    VendorSpecificExtension? serviceSpecificDataStoredInEuicc,
    List<NotificationConfigurationInformation>? notificationConfigurationInfo,
    Uint8List? tagsForDeletion,
    RpmConfiguration? rpmConfiguration,
    String? hriServerAddress,
    LprConfiguration? lprConfiguration,
    EnterpriseConfiguration? enterpriseConfiguration,
    DeviceChangeConfiguration? deviceChangeConfiguration,
  }) : this._(
         serviceProviderName: serviceProviderName,
         profileName: profileName,
         iconType: iconType,
         icon: icon,
         profilePolicyRules: profilePolicyRules,
         serviceSpecificDataStoredInEuicc: serviceSpecificDataStoredInEuicc,
         notificationConfigurationInfo: notificationConfigurationInfo,
         tagsForDeletion: tagsForDeletion,
         rpmConfiguration: rpmConfiguration,
         hriServerAddress: hriServerAddress,
         lprConfiguration: lprConfiguration,
         enterpriseConfiguration: enterpriseConfiguration,
         deviceChangeConfiguration: deviceChangeConfiguration,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF2A;
    if (serviceProviderName != null)
      children.add(
        _asn1EncodeRaw(
          0x91,
          Uint8List.fromList(serviceProviderName!.codeUnits),
        ),
      );
    if (profileName != null)
      children.add(
        _asn1EncodeRaw(0x92, Uint8List.fromList(profileName!.codeUnits)),
      );
    if (iconType != null)
      children.add(_asn1EncodeRaw(0x93, _asn1EncodeInt(iconType!.value)));
    if (icon != null) children.add(_asn1EncodeRaw(0x94, icon!));
    if (profilePolicyRules != null)
      children.add(
        _asn1EncodeRaw(0x99, _asn1EncodeBits(profilePolicyRules!.value)),
      );
    if (serviceSpecificDataStoredInEuicc != null) {
      final items = <Uint8List>[];
      for (final v in serviceSpecificDataStoredInEuicc!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xBF22, items));
    }
    if (notificationConfigurationInfo != null) {
      final items = <Uint8List>[];
      for (final v in notificationConfigurationInfo!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xB6, items));
    }
    if (tagsForDeletion != null)
      children.add(_asn1EncodeRaw(0x5C, tagsForDeletion!));
    if (rpmConfiguration != null)
      children.add((rpmConfiguration as dynamic).encode(tag: 0xBA));
    if (hriServerAddress != null)
      children.add(
        _asn1EncodeRaw(0x9B, Uint8List.fromList(hriServerAddress!.codeUnits)),
      );
    if (lprConfiguration != null)
      children.add((lprConfiguration as dynamic).encode(tag: 0xBC));
    if (enterpriseConfiguration != null)
      children.add((enterpriseConfiguration as dynamic).encode(tag: 0xBD));
    if (deviceChangeConfiguration != null)
      children.add(
        _asn1EncodeConstructed(0xBF20, [
          (deviceChangeConfiguration as dynamic).encode(),
        ]),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory UpdateMetadataRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF2A;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    String? serviceProviderName;
    String? profileName;
    IconType? iconType;
    Uint8List? icon;
    PprIds? profilePolicyRules;
    VendorSpecificExtension? serviceSpecificDataStoredInEuicc;
    List<NotificationConfigurationInformation>? notificationConfigurationInfo;
    Uint8List? tagsForDeletion;
    RpmConfiguration? rpmConfiguration;
    String? hriServerAddress;
    LprConfiguration? lprConfiguration;
    EnterpriseConfiguration? enterpriseConfiguration;
    DeviceChangeConfiguration? deviceChangeConfiguration;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x91);
    if (found >= 0) {
      final child = node!.children[found];
      serviceProviderName = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x92);
    if (found >= 0) {
      final child = node!.children[found];
      profileName = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x93);
    if (found >= 0) {
      final child = node!.children[found];
      iconType = IconTypeExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x94);
    if (found >= 0) {
      final child = node!.children[found];
      icon = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x99);
    if (found >= 0) {
      final child = node!.children[found];
      profilePolicyRules = PprIds(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF22);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <VendorSpecificExtension__item_>[];
      for (final n in child.children) {
        items.add(VendorSpecificExtension__item_.decode(n));
      }
      serviceSpecificDataStoredInEuicc = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB6);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <NotificationConfigurationInformation>[];
      for (final n in child.children) {
        items.add(NotificationConfigurationInformation.decode(n));
      }
      notificationConfigurationInfo = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5C);
    if (found >= 0) {
      final child = node!.children[found];
      tagsForDeletion = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBA);
    if (found >= 0) {
      final child = node!.children[found];
      rpmConfiguration = RpmConfiguration.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9B);
    if (found >= 0) {
      final child = node!.children[found];
      hriServerAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBC);
    if (found >= 0) {
      final child = node!.children[found];
      lprConfiguration = LprConfiguration.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBD);
    if (found >= 0) {
      final child = node!.children[found];
      enterpriseConfiguration = EnterpriseConfiguration.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF20);
    if (found >= 0) {
      final child = node!.children[found];
      deviceChangeConfiguration = DeviceChangeConfiguration.decode(
        child.children.first,
      );
      childIdx = found + 1;
    }
    return UpdateMetadataRequest._(
      serviceProviderName: serviceProviderName,
      profileName: profileName,
      iconType: iconType,
      icon: icon,
      profilePolicyRules: profilePolicyRules,
      serviceSpecificDataStoredInEuicc: serviceSpecificDataStoredInEuicc,
      notificationConfigurationInfo: notificationConfigurationInfo,
      tagsForDeletion: tagsForDeletion,
      rpmConfiguration: rpmConfiguration,
      hriServerAddress: hriServerAddress,
      lprConfiguration: lprConfiguration,
      enterpriseConfiguration: enterpriseConfiguration,
      deviceChangeConfiguration: deviceChangeConfiguration,
      asn1: node, // Store original node
    );
  }

  static UpdateMetadataRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return UpdateMetadataRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return UpdateMetadataRequest.fromAsn1(node, tag: tag);
    }
    return UpdateMetadataRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('UpdateMetadataRequest(');
    final serviceProviderNameVal = serviceProviderName;
    if (serviceProviderNameVal != null) {
      final val = serviceProviderNameVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceProviderName: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serviceProviderName: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileNameVal = profileName;
    if (profileNameVal != null) {
      final val = profileNameVal;
      if (val is Uint8List) {
        buffer.writeln('  profileName: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileName: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iconTypeVal = iconType;
    if (iconTypeVal != null) {
      final val = iconTypeVal;
      buffer.writeln('  iconType: $val,');
    }
    final iconVal = icon;
    if (iconVal != null) {
      final val = iconVal;
      buffer.writeln('  icon: ${val.length} bytes,');
    }
    final profilePolicyRulesVal = profilePolicyRules;
    if (profilePolicyRulesVal != null) {
      final val = profilePolicyRulesVal;
      if (val is Uint8List) {
        buffer.writeln('  profilePolicyRules: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profilePolicyRules: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceSpecificDataStoredInEuiccVal =
        serviceSpecificDataStoredInEuicc;
    if (serviceSpecificDataStoredInEuiccVal != null) {
      final val = serviceSpecificDataStoredInEuiccVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceSpecificDataStoredInEuicc: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  serviceSpecificDataStoredInEuicc: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  serviceSpecificDataStoredInEuicc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notificationConfigurationInfoVal = notificationConfigurationInfo;
    if (notificationConfigurationInfoVal != null) {
      final val = notificationConfigurationInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationConfigurationInfo: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  notificationConfigurationInfo: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  notificationConfigurationInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final tagsForDeletionVal = tagsForDeletion;
    if (tagsForDeletionVal != null) {
      final val = tagsForDeletionVal;
      if (val is Uint8List) {
        buffer.writeln('  tagsForDeletion: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tagsForDeletion: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmConfigurationVal = rpmConfiguration;
    if (rpmConfigurationVal != null) {
      final val = rpmConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final hriServerAddressVal = hriServerAddress;
    if (hriServerAddressVal != null) {
      final val = hriServerAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  hriServerAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  hriServerAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final lprConfigurationVal = lprConfiguration;
    if (lprConfigurationVal != null) {
      final val = lprConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  lprConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  lprConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final enterpriseConfigurationVal = enterpriseConfiguration;
    if (enterpriseConfigurationVal != null) {
      final val = enterpriseConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  enterpriseConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enterpriseConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deviceChangeConfigurationVal = deviceChangeConfiguration;
    if (deviceChangeConfigurationVal != null) {
      final val = deviceChangeConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceChangeConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceChangeConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class InitialiseSecureChannelRequest {
  final RemoteOpId? remoteOpId;
  final TransactionId? transactionId;
  final ControlRefTemplate? controlRefTemplate;
  final Uint8List? smdpOtpk;
  final Uint8List? smdpSign;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const InitialiseSecureChannelRequest._({
    this.remoteOpId,
    this.transactionId,
    this.controlRefTemplate,
    this.smdpOtpk,
    this.smdpSign,
    this.asn1,
  });

  const InitialiseSecureChannelRequest({
    RemoteOpId? remoteOpId,
    TransactionId? transactionId,
    ControlRefTemplate? controlRefTemplate,
    Uint8List? smdpOtpk,
    Uint8List? smdpSign,
  }) : this._(
         remoteOpId: remoteOpId,
         transactionId: transactionId,
         controlRefTemplate: controlRefTemplate,
         smdpOtpk: smdpOtpk,
         smdpSign: smdpSign,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF23;
    if (remoteOpId != null)
      children.add(_asn1EncodeRaw(0x82, _asn1EncodeInt(remoteOpId!.value)));
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (controlRefTemplate != null)
      children.add((controlRefTemplate as dynamic).encode(tag: 0xA6));
    if (smdpOtpk != null) children.add(_asn1EncodeRaw(0x5F49, smdpOtpk!));
    if (smdpSign != null) children.add(_asn1EncodeRaw(0x5F37, smdpSign!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory InitialiseSecureChannelRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF23;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    RemoteOpId? remoteOpId;
    TransactionId? transactionId;
    ControlRefTemplate? controlRefTemplate;
    Uint8List? smdpOtpk;
    Uint8List? smdpSign;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      remoteOpId = RemoteOpIdExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA6);
    if (found >= 0) {
      final child = node!.children[found];
      controlRefTemplate = ControlRefTemplate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F49);
    if (found >= 0) {
      final child = node!.children[found];
      smdpOtpk = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSign = child.value;
      childIdx = found + 1;
    }
    return InitialiseSecureChannelRequest._(
      remoteOpId: remoteOpId,
      transactionId: transactionId,
      controlRefTemplate: controlRefTemplate,
      smdpOtpk: smdpOtpk,
      smdpSign: smdpSign,
      asn1: node, // Store original node
    );
  }

  static InitialiseSecureChannelRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return InitialiseSecureChannelRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return InitialiseSecureChannelRequest.fromAsn1(node, tag: tag);
    }
    return InitialiseSecureChannelRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('InitialiseSecureChannelRequest(');
    final remoteOpIdVal = remoteOpId;
    if (remoteOpIdVal != null) {
      final val = remoteOpIdVal;
      if (val is Uint8List) {
        buffer.writeln('  remoteOpId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  remoteOpId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final controlRefTemplateVal = controlRefTemplate;
    if (controlRefTemplateVal != null) {
      final val = controlRefTemplateVal;
      if (val is Uint8List) {
        buffer.writeln('  controlRefTemplate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  controlRefTemplate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpOtpkVal = smdpOtpk;
    if (smdpOtpkVal != null) {
      final val = smdpOtpkVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpOtpk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpOtpk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSignVal = smdpSign;
    if (smdpSignVal != null) {
      final val = smdpSignVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpSign: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSign: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ControlRefTemplate {
  final Octet1? keyType;
  final Octet1? keyLen;
  final OctetTo16? hostId;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ControlRefTemplate._({
    this.keyType,
    this.keyLen,
    this.hostId,
    this.asn1,
  });

  const ControlRefTemplate({Octet1? keyType, Octet1? keyLen, OctetTo16? hostId})
    : this._(keyType: keyType, keyLen: keyLen, hostId: hostId, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (keyType != null) children.add(_asn1EncodeRaw(0x80, keyType!));
    if (keyLen != null) children.add(_asn1EncodeRaw(0x81, keyLen!));
    if (hostId != null) children.add(_asn1EncodeRaw(0x84, hostId!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ControlRefTemplate.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Octet1? keyType;
    Octet1? keyLen;
    OctetTo16? hostId;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      keyType = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      keyLen = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x84);
    if (found >= 0) {
      final child = node!.children[found];
      hostId = child.value;
      childIdx = found + 1;
    }
    return ControlRefTemplate._(
      keyType: keyType,
      keyLen: keyLen,
      hostId: hostId,
      asn1: node, // Store original node
    );
  }

  static ControlRefTemplate decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return ControlRefTemplate.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ControlRefTemplate.fromAsn1(node, tag: tag);
    }
    return ControlRefTemplate();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ControlRefTemplate(');
    final keyTypeVal = keyType;
    if (keyTypeVal != null) {
      final val = keyTypeVal;
      if (val is Uint8List) {
        buffer.writeln('  keyType: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  keyType: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final keyLenVal = keyLen;
    if (keyLenVal != null) {
      final val = keyLenVal;
      if (val is Uint8List) {
        buffer.writeln('  keyLen: ${_hex(val)},');
      } else {
        buffer.writeln('  keyLen: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final hostIdVal = hostId;
    if (hostIdVal != null) {
      final val = hostIdVal;
      if (val is Uint8List) {
        buffer.writeln('  hostId: ${_hex(val)},');
      } else {
        buffer.writeln('  hostId: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DpProprietaryData {
  final Uint8List? dpOid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DpProprietaryData._({this.dpOid, this.asn1});

  const DpProprietaryData({Uint8List? dpOid})
    : this._(dpOid: dpOid, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (dpOid != null) children.add(_asn1EncodeRaw(0x80, dpOid!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DpProprietaryData.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? dpOid;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      dpOid = child.value;
      childIdx = found + 1;
    }
    return DpProprietaryData._(
      dpOid: dpOid,
      asn1: node, // Store original node
    );
  }

  static DpProprietaryData decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return DpProprietaryData.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return DpProprietaryData.fromAsn1(node, tag: tag);
    }
    return DpProprietaryData();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DpProprietaryData(');
    final dpOidVal = dpOid;
    if (dpOidVal != null) {
      final val = dpOidVal;
      if (val is Uint8List) {
        buffer.writeln('  dpOid: ${_hex(val)},');
      } else {
        buffer.writeln('  dpOid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class StoreMetadataRequest {
  final Iccid? iccid;
  final String? serviceProviderName;
  final String? profileName;
  final IconType? iconType;
  final Uint8List? icon;
  final ProfileClass? profileClass;
  final List<NotificationConfigurationInformation>?
  notificationConfigurationInfo;
  final OperatorId? profileOwner;
  final PprIds? profilePolicyRules;
  final VendorSpecificExtension? serviceSpecificDataStoredInEuicc;
  final VendorSpecificExtension? serviceSpecificDataNotStoredInEuicc;
  final RpmConfiguration? rpmConfiguration;
  final String? hriServerAddress;
  final LocalisedTextMessage? serviceProviderMessage;
  final LprConfiguration? lprConfiguration;
  final EnterpriseConfiguration? enterpriseConfiguration;
  final ServiceDescription? serviceDescription;
  final DeviceChangeConfiguration? deviceChangeConfiguration;
  final int? estimatedProfileSize;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const StoreMetadataRequest._({
    this.iccid,
    this.serviceProviderName,
    this.profileName,
    this.iconType,
    this.icon,
    this.profileClass,
    this.notificationConfigurationInfo,
    this.profileOwner,
    this.profilePolicyRules,
    this.serviceSpecificDataStoredInEuicc,
    this.serviceSpecificDataNotStoredInEuicc,
    this.rpmConfiguration,
    this.hriServerAddress,
    this.serviceProviderMessage,
    this.lprConfiguration,
    this.enterpriseConfiguration,
    this.serviceDescription,
    this.deviceChangeConfiguration,
    this.estimatedProfileSize,
    this.asn1,
  });

  const StoreMetadataRequest({
    Iccid? iccid,
    String? serviceProviderName,
    String? profileName,
    IconType? iconType,
    Uint8List? icon,
    ProfileClass? profileClass,
    List<NotificationConfigurationInformation>? notificationConfigurationInfo,
    OperatorId? profileOwner,
    PprIds? profilePolicyRules,
    VendorSpecificExtension? serviceSpecificDataStoredInEuicc,
    VendorSpecificExtension? serviceSpecificDataNotStoredInEuicc,
    RpmConfiguration? rpmConfiguration,
    String? hriServerAddress,
    LocalisedTextMessage? serviceProviderMessage,
    LprConfiguration? lprConfiguration,
    EnterpriseConfiguration? enterpriseConfiguration,
    ServiceDescription? serviceDescription,
    DeviceChangeConfiguration? deviceChangeConfiguration,
    int? estimatedProfileSize,
  }) : this._(
         iccid: iccid,
         serviceProviderName: serviceProviderName,
         profileName: profileName,
         iconType: iconType,
         icon: icon,
         profileClass: profileClass,
         notificationConfigurationInfo: notificationConfigurationInfo,
         profileOwner: profileOwner,
         profilePolicyRules: profilePolicyRules,
         serviceSpecificDataStoredInEuicc: serviceSpecificDataStoredInEuicc,
         serviceSpecificDataNotStoredInEuicc:
             serviceSpecificDataNotStoredInEuicc,
         rpmConfiguration: rpmConfiguration,
         hriServerAddress: hriServerAddress,
         serviceProviderMessage: serviceProviderMessage,
         lprConfiguration: lprConfiguration,
         enterpriseConfiguration: enterpriseConfiguration,
         serviceDescription: serviceDescription,
         deviceChangeConfiguration: deviceChangeConfiguration,
         estimatedProfileSize: estimatedProfileSize,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF25;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (serviceProviderName != null)
      children.add(
        _asn1EncodeRaw(
          0x91,
          Uint8List.fromList(serviceProviderName!.codeUnits),
        ),
      );
    if (profileName != null)
      children.add(
        _asn1EncodeRaw(0x92, Uint8List.fromList(profileName!.codeUnits)),
      );
    if (iconType != null)
      children.add(_asn1EncodeRaw(0x93, _asn1EncodeInt(iconType!.value)));
    if (icon != null) children.add(_asn1EncodeRaw(0x94, icon!));
    if (profileClass != null)
      children.add(_asn1EncodeRaw(0x95, _asn1EncodeInt(profileClass!.value)));
    if (notificationConfigurationInfo != null) {
      final items = <Uint8List>[];
      for (final v in notificationConfigurationInfo!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xB6, items));
    }
    if (profileOwner != null)
      children.add((profileOwner as dynamic).encode(tag: 0xB7));
    if (profilePolicyRules != null)
      children.add(
        _asn1EncodeRaw(0x99, _asn1EncodeBits(profilePolicyRules!.value)),
      );
    if (serviceSpecificDataStoredInEuicc != null) {
      final items = <Uint8List>[];
      for (final v in serviceSpecificDataStoredInEuicc!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xBF22, items));
    }
    if (serviceSpecificDataNotStoredInEuicc != null) {
      final items = <Uint8List>[];
      for (final v in serviceSpecificDataNotStoredInEuicc!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xBF23, items));
    }
    if (rpmConfiguration != null)
      children.add((rpmConfiguration as dynamic).encode(tag: 0xBA));
    if (hriServerAddress != null)
      children.add(
        _asn1EncodeRaw(0x9B, Uint8List.fromList(hriServerAddress!.codeUnits)),
      );
    if (serviceProviderMessage != null)
      children.add((serviceProviderMessage as dynamic).encode(tag: 0xBE));
    if (lprConfiguration != null)
      children.add((lprConfiguration as dynamic).encode(tag: 0xBC));
    if (enterpriseConfiguration != null)
      children.add((enterpriseConfiguration as dynamic).encode(tag: 0xBD));
    if (serviceDescription != null)
      children.add(
        _asn1EncodeRaw(0x9F1F, _asn1EncodeBits(serviceDescription!.value)),
      );
    if (deviceChangeConfiguration != null)
      children.add(
        _asn1EncodeConstructed(0xBF20, [
          (deviceChangeConfiguration as dynamic).encode(),
        ]),
      );
    if (estimatedProfileSize != null)
      children.add(
        _asn1EncodeRaw(0x9F21, _asn1EncodeInt(estimatedProfileSize!)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory StoreMetadataRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF25;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    String? serviceProviderName;
    String? profileName;
    IconType? iconType;
    Uint8List? icon;
    ProfileClass? profileClass;
    List<NotificationConfigurationInformation>? notificationConfigurationInfo;
    OperatorId? profileOwner;
    PprIds? profilePolicyRules;
    VendorSpecificExtension? serviceSpecificDataStoredInEuicc;
    VendorSpecificExtension? serviceSpecificDataNotStoredInEuicc;
    RpmConfiguration? rpmConfiguration;
    String? hriServerAddress;
    LocalisedTextMessage? serviceProviderMessage;
    LprConfiguration? lprConfiguration;
    EnterpriseConfiguration? enterpriseConfiguration;
    ServiceDescription? serviceDescription;
    DeviceChangeConfiguration? deviceChangeConfiguration;
    int? estimatedProfileSize;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x91);
    if (found >= 0) {
      final child = node!.children[found];
      serviceProviderName = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x92);
    if (found >= 0) {
      final child = node!.children[found];
      profileName = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x93);
    if (found >= 0) {
      final child = node!.children[found];
      iconType = IconTypeExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x94);
    if (found >= 0) {
      final child = node!.children[found];
      icon = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x95);
    if (found >= 0) {
      final child = node!.children[found];
      profileClass = ProfileClassExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB6);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <NotificationConfigurationInformation>[];
      for (final n in child.children) {
        items.add(NotificationConfigurationInformation.decode(n));
      }
      notificationConfigurationInfo = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB7);
    if (found >= 0) {
      final child = node!.children[found];
      profileOwner = OperatorId.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x99);
    if (found >= 0) {
      final child = node!.children[found];
      profilePolicyRules = PprIds(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF22);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <VendorSpecificExtension__item_>[];
      for (final n in child.children) {
        items.add(VendorSpecificExtension__item_.decode(n));
      }
      serviceSpecificDataStoredInEuicc = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF23);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <VendorSpecificExtension__item_>[];
      for (final n in child.children) {
        items.add(VendorSpecificExtension__item_.decode(n));
      }
      serviceSpecificDataNotStoredInEuicc = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBA);
    if (found >= 0) {
      final child = node!.children[found];
      rpmConfiguration = RpmConfiguration.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9B);
    if (found >= 0) {
      final child = node!.children[found];
      hriServerAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBE);
    if (found >= 0) {
      final child = node!.children[found];
      serviceProviderMessage = LocalisedTextMessage.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBC);
    if (found >= 0) {
      final child = node!.children[found];
      lprConfiguration = LprConfiguration.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBD);
    if (found >= 0) {
      final child = node!.children[found];
      enterpriseConfiguration = EnterpriseConfiguration.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9F1F);
    if (found >= 0) {
      final child = node!.children[found];
      serviceDescription = ServiceDescription(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF20);
    if (found >= 0) {
      final child = node!.children[found];
      deviceChangeConfiguration = DeviceChangeConfiguration.decode(
        child.children.first,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9F21);
    if (found >= 0) {
      final child = node!.children[found];
      estimatedProfileSize = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return StoreMetadataRequest._(
      iccid: iccid,
      serviceProviderName: serviceProviderName,
      profileName: profileName,
      iconType: iconType,
      icon: icon,
      profileClass: profileClass,
      notificationConfigurationInfo: notificationConfigurationInfo,
      profileOwner: profileOwner,
      profilePolicyRules: profilePolicyRules,
      serviceSpecificDataStoredInEuicc: serviceSpecificDataStoredInEuicc,
      serviceSpecificDataNotStoredInEuicc: serviceSpecificDataNotStoredInEuicc,
      rpmConfiguration: rpmConfiguration,
      hriServerAddress: hriServerAddress,
      serviceProviderMessage: serviceProviderMessage,
      lprConfiguration: lprConfiguration,
      enterpriseConfiguration: enterpriseConfiguration,
      serviceDescription: serviceDescription,
      deviceChangeConfiguration: deviceChangeConfiguration,
      estimatedProfileSize: estimatedProfileSize,
      asn1: node, // Store original node
    );
  }

  static StoreMetadataRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return StoreMetadataRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return StoreMetadataRequest.fromAsn1(node, tag: tag);
    }
    return StoreMetadataRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('StoreMetadataRequest(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final serviceProviderNameVal = serviceProviderName;
    if (serviceProviderNameVal != null) {
      final val = serviceProviderNameVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceProviderName: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serviceProviderName: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileNameVal = profileName;
    if (profileNameVal != null) {
      final val = profileNameVal;
      if (val is Uint8List) {
        buffer.writeln('  profileName: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileName: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iconTypeVal = iconType;
    if (iconTypeVal != null) {
      final val = iconTypeVal;
      buffer.writeln('  iconType: $val,');
    }
    final iconVal = icon;
    if (iconVal != null) {
      final val = iconVal;
      buffer.writeln('  icon: ${val.length} bytes,');
    }
    final profileClassVal = profileClass;
    if (profileClassVal != null) {
      final val = profileClassVal;
      if (val is Uint8List) {
        buffer.writeln('  profileClass: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileClass: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notificationConfigurationInfoVal = notificationConfigurationInfo;
    if (notificationConfigurationInfoVal != null) {
      final val = notificationConfigurationInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationConfigurationInfo: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  notificationConfigurationInfo: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  notificationConfigurationInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileOwnerVal = profileOwner;
    if (profileOwnerVal != null) {
      final val = profileOwnerVal;
      if (val is Uint8List) {
        buffer.writeln('  profileOwner: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileOwner: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profilePolicyRulesVal = profilePolicyRules;
    if (profilePolicyRulesVal != null) {
      final val = profilePolicyRulesVal;
      if (val is Uint8List) {
        buffer.writeln('  profilePolicyRules: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profilePolicyRules: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceSpecificDataStoredInEuiccVal =
        serviceSpecificDataStoredInEuicc;
    if (serviceSpecificDataStoredInEuiccVal != null) {
      final val = serviceSpecificDataStoredInEuiccVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceSpecificDataStoredInEuicc: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  serviceSpecificDataStoredInEuicc: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  serviceSpecificDataStoredInEuicc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceSpecificDataNotStoredInEuiccVal =
        serviceSpecificDataNotStoredInEuicc;
    if (serviceSpecificDataNotStoredInEuiccVal != null) {
      final val = serviceSpecificDataNotStoredInEuiccVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceSpecificDataNotStoredInEuicc: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  serviceSpecificDataNotStoredInEuicc: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  serviceSpecificDataNotStoredInEuicc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmConfigurationVal = rpmConfiguration;
    if (rpmConfigurationVal != null) {
      final val = rpmConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final hriServerAddressVal = hriServerAddress;
    if (hriServerAddressVal != null) {
      final val = hriServerAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  hriServerAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  hriServerAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceProviderMessageVal = serviceProviderMessage;
    if (serviceProviderMessageVal != null) {
      final val = serviceProviderMessageVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceProviderMessage: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serviceProviderMessage: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final lprConfigurationVal = lprConfiguration;
    if (lprConfigurationVal != null) {
      final val = lprConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  lprConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  lprConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final enterpriseConfigurationVal = enterpriseConfiguration;
    if (enterpriseConfigurationVal != null) {
      final val = enterpriseConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  enterpriseConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enterpriseConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceDescriptionVal = serviceDescription;
    if (serviceDescriptionVal != null) {
      final val = serviceDescriptionVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceDescription: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serviceDescription: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deviceChangeConfigurationVal = deviceChangeConfiguration;
    if (deviceChangeConfigurationVal != null) {
      final val = deviceChangeConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceChangeConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceChangeConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final estimatedProfileSizeVal = estimatedProfileSize;
    if (estimatedProfileSizeVal != null) {
      final val = estimatedProfileSizeVal;
      if (val is Uint8List) {
        buffer.writeln('  estimatedProfileSize: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  estimatedProfileSize: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class NotificationConfigurationInformation {
  final NotificationEvent? profileManagementOperation;
  final String? notificationAddress;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const NotificationConfigurationInformation._({
    this.profileManagementOperation,
    this.notificationAddress,
    this.asn1,
  });

  const NotificationConfigurationInformation({
    NotificationEvent? profileManagementOperation,
    String? notificationAddress,
  }) : this._(
         profileManagementOperation: profileManagementOperation,
         notificationAddress: notificationAddress,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (profileManagementOperation != null)
      children.add(
        _asn1EncodeRaw(
          0x80,
          _asn1EncodeBits(profileManagementOperation!.value),
        ),
      );
    if (notificationAddress != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          Uint8List.fromList(notificationAddress!.codeUnits),
        ),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory NotificationConfigurationInformation.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0x30;
    NotificationEvent? profileManagementOperation;
    String? notificationAddress;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      profileManagementOperation = NotificationEvent(
        _asn1DecodeBits(child.value),
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      notificationAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return NotificationConfigurationInformation._(
      profileManagementOperation: profileManagementOperation,
      notificationAddress: notificationAddress,
      asn1: node, // Store original node
    );
  }

  static NotificationConfigurationInformation decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return NotificationConfigurationInformation.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return NotificationConfigurationInformation.fromAsn1(node, tag: tag);
    }
    return NotificationConfigurationInformation();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('NotificationConfigurationInformation(');
    final profileManagementOperationVal = profileManagementOperation;
    if (profileManagementOperationVal != null) {
      final val = profileManagementOperationVal;
      if (val is Uint8List) {
        buffer.writeln('  profileManagementOperation: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileManagementOperation: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notificationAddressVal = notificationAddress;
    if (notificationAddressVal != null) {
      final val = notificationAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  notificationAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EuiccConfiguredDataRequest {
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EuiccConfiguredDataRequest._({this.asn1});
  const EuiccConfiguredDataRequest() : this._(asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF3C;
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EuiccConfiguredDataRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF3C;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int childIdx = 0;
    return EuiccConfiguredDataRequest._(
      asn1: node, // Store original node
    );
  }

  static EuiccConfiguredDataRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return EuiccConfiguredDataRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return EuiccConfiguredDataRequest.fromAsn1(node, tag: tag);
    }
    return EuiccConfiguredDataRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EuiccConfiguredDataRequest(');
    buffer.write(')');
    return buffer.toString();
  }
}

class EuiccConfiguredDataResponse {
  final String? defaultDpAddress;
  final String? rootDsAddress;
  final List<Uint8List>? additionalRootDsAddresses;
  final SubjectKeyIdentifier? allowedCiPKId;
  final List<EuiccConfiguredDataResponse_ciList__item_>? ciList;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EuiccConfiguredDataResponse._({
    this.defaultDpAddress,
    this.rootDsAddress,
    this.additionalRootDsAddresses,
    this.allowedCiPKId,
    this.ciList,
    this.asn1,
  });

  const EuiccConfiguredDataResponse({
    String? defaultDpAddress,
    String? rootDsAddress,
    List<Uint8List>? additionalRootDsAddresses,
    SubjectKeyIdentifier? allowedCiPKId,
    List<EuiccConfiguredDataResponse_ciList__item_>? ciList,
  }) : this._(
         defaultDpAddress: defaultDpAddress,
         rootDsAddress: rootDsAddress,
         additionalRootDsAddresses: additionalRootDsAddresses,
         allowedCiPKId: allowedCiPKId,
         ciList: ciList,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF3C;
    if (defaultDpAddress != null)
      children.add(
        _asn1EncodeRaw(0x80, Uint8List.fromList(defaultDpAddress!.codeUnits)),
      );
    if (rootDsAddress != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList(rootDsAddress!.codeUnits)),
      );
    if (additionalRootDsAddresses != null) {
      final items = <Uint8List>[];
      for (final v in additionalRootDsAddresses!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xA2, items));
    }
    if (allowedCiPKId != null)
      children.add(_asn1EncodeRaw(0x83, allowedCiPKId!));
    if (ciList != null) {
      final items = <Uint8List>[];
      for (final v in ciList!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA4, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EuiccConfiguredDataResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF3C;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    String? defaultDpAddress;
    String? rootDsAddress;
    List<Uint8List>? additionalRootDsAddresses;
    SubjectKeyIdentifier? allowedCiPKId;
    List<EuiccConfiguredDataResponse_ciList__item_>? ciList;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      defaultDpAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      rootDsAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA2);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Uint8List>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      additionalRootDsAddresses = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      allowedCiPKId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA4);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <EuiccConfiguredDataResponse_ciList__item_>[];
      for (final n in child.children) {
        items.add(EuiccConfiguredDataResponse_ciList__item_.decode(n));
      }
      ciList = items;
      childIdx = found + 1;
    }
    return EuiccConfiguredDataResponse._(
      defaultDpAddress: defaultDpAddress,
      rootDsAddress: rootDsAddress,
      additionalRootDsAddresses: additionalRootDsAddresses,
      allowedCiPKId: allowedCiPKId,
      ciList: ciList,
      asn1: node, // Store original node
    );
  }

  static EuiccConfiguredDataResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return EuiccConfiguredDataResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return EuiccConfiguredDataResponse.fromAsn1(node, tag: tag);
    }
    return EuiccConfiguredDataResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EuiccConfiguredDataResponse(');
    final defaultDpAddressVal = defaultDpAddress;
    if (defaultDpAddressVal != null) {
      final val = defaultDpAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  defaultDpAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  defaultDpAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rootDsAddressVal = rootDsAddress;
    if (rootDsAddressVal != null) {
      final val = rootDsAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  rootDsAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rootDsAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final additionalRootDsAddressesVal = additionalRootDsAddresses;
    if (additionalRootDsAddressesVal != null) {
      final val = additionalRootDsAddressesVal;
      if (val is Uint8List) {
        buffer.writeln('  additionalRootDsAddresses: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  additionalRootDsAddresses: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  additionalRootDsAddresses: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final allowedCiPKIdVal = allowedCiPKId;
    if (allowedCiPKIdVal != null) {
      final val = allowedCiPKIdVal;
      if (val is Uint8List) {
        buffer.writeln('  allowedCiPKId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  allowedCiPKId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ciListVal = ciList;
    if (ciListVal != null) {
      final val = ciListVal;
      if (val is Uint8List) {
        buffer.writeln('  ciList: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  ciList: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln('  ciList: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EuiccConfiguredDataResponse_ciList__item_ {
  final SubjectKeyIdentifier? ciPKId;
  final String? ciName;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EuiccConfiguredDataResponse_ciList__item_._({
    this.ciPKId,
    this.ciName,
    this.asn1,
  });

  const EuiccConfiguredDataResponse_ciList__item_({
    SubjectKeyIdentifier? ciPKId,
    String? ciName,
  }) : this._(ciPKId: ciPKId, ciName: ciName, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (ciPKId != null) children.add(_asn1EncodeRaw(0x80, ciPKId!));
    if (ciName != null)
      children.add(_asn1EncodeRaw(0x81, Uint8List.fromList(ciName!.codeUnits)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EuiccConfiguredDataResponse_ciList__item_.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0x30;
    SubjectKeyIdentifier? ciPKId;
    String? ciName;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      ciPKId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      ciName = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return EuiccConfiguredDataResponse_ciList__item_._(
      ciPKId: ciPKId,
      ciName: ciName,
      asn1: node, // Store original node
    );
  }

  static EuiccConfiguredDataResponse_ciList__item_ decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return EuiccConfiguredDataResponse_ciList__item_.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return EuiccConfiguredDataResponse_ciList__item_.fromAsn1(
          node,
          tag: tag,
        );
    }
    return EuiccConfiguredDataResponse_ciList__item_();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EuiccConfiguredDataResponse_ciList__item_(');
    final ciPKIdVal = ciPKId;
    if (ciPKIdVal != null) {
      final val = ciPKIdVal;
      if (val is Uint8List) {
        buffer.writeln('  ciPKId: ${_hex(val)},');
      } else {
        buffer.writeln('  ciPKId: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final ciNameVal = ciName;
    if (ciNameVal != null) {
      final val = ciNameVal;
      if (val is Uint8List) {
        buffer.writeln('  ciName: ${_hex(val)},');
      } else {
        buffer.writeln('  ciName: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class PrepareDownloadRequest {
  final SmdpSigned2? smdpSigned2;
  final Uint8List? smdpSignature2;
  final Octet32? hashCc;
  final Certificate? smdpCertificate;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const PrepareDownloadRequest._({
    this.smdpSigned2,
    this.smdpSignature2,
    this.hashCc,
    this.smdpCertificate,
    this.asn1,
  });

  const PrepareDownloadRequest({
    SmdpSigned2? smdpSigned2,
    Uint8List? smdpSignature2,
    Octet32? hashCc,
    Certificate? smdpCertificate,
  }) : this._(
         smdpSigned2: smdpSigned2,
         smdpSignature2: smdpSignature2,
         hashCc: hashCc,
         smdpCertificate: smdpCertificate,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF21;
    if (smdpSigned2 != null)
      children.add((smdpSigned2 as dynamic).encode(tag: 0x30));
    if (smdpSignature2 != null)
      children.add(_asn1EncodeRaw(0x5F37, smdpSignature2!));
    if (hashCc != null) children.add(_asn1EncodeRaw(0x4, hashCc!));
    if (smdpCertificate != null)
      children.add((smdpCertificate as dynamic).encode(tag: 0x30));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory PrepareDownloadRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF21;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    SmdpSigned2? smdpSigned2;
    Uint8List? smdpSignature2;
    Octet32? hashCc;
    Certificate? smdpCertificate;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSigned2 = SmdpSigned2.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSignature2 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      hashCc = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      smdpCertificate = Certificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    return PrepareDownloadRequest._(
      smdpSigned2: smdpSigned2,
      smdpSignature2: smdpSignature2,
      hashCc: hashCc,
      smdpCertificate: smdpCertificate,
      asn1: node, // Store original node
    );
  }

  static PrepareDownloadRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return PrepareDownloadRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return PrepareDownloadRequest.fromAsn1(node, tag: tag);
    }
    return PrepareDownloadRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('PrepareDownloadRequest(');
    final smdpSigned2Val = smdpSigned2;
    if (smdpSigned2Val != null) {
      final val = smdpSigned2Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSigned2: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSigned2: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSignature2Val = smdpSignature2;
    if (smdpSignature2Val != null) {
      final val = smdpSignature2Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSignature2: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSignature2: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final hashCcVal = hashCc;
    if (hashCcVal != null) {
      final val = hashCcVal;
      if (val is Uint8List) {
        buffer.writeln('  hashCc: ${_hex(val)},');
      } else {
        buffer.writeln('  hashCc: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final smdpCertificateVal = smdpCertificate;
    if (smdpCertificateVal != null) {
      final val = smdpCertificateVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpCertificate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpCertificate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SmdpSigned2 {
  final TransactionId? transactionId;
  final bool? ccRequiredFlag;
  final Uint8List? bppEuiccOtpk;
  final Uint8List? rpmPending;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SmdpSigned2._({
    this.transactionId,
    this.ccRequiredFlag,
    this.bppEuiccOtpk,
    this.rpmPending,
    this.asn1,
  });

  const SmdpSigned2({
    TransactionId? transactionId,
    bool? ccRequiredFlag,
    Uint8List? bppEuiccOtpk,
    Uint8List? rpmPending,
  }) : this._(
         transactionId: transactionId,
         ccRequiredFlag: ccRequiredFlag,
         bppEuiccOtpk: bppEuiccOtpk,
         rpmPending: rpmPending,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (ccRequiredFlag != null)
      children.add(
        _asn1EncodeRaw(
          0x1,
          Uint8List.fromList([ccRequiredFlag! ? 0xFF : 0x00]),
        ),
      );
    if (bppEuiccOtpk != null)
      children.add(_asn1EncodeRaw(0x5F49, bppEuiccOtpk!));
    if (rpmPending != null) children.add(_asn1EncodeRaw(0x4, rpmPending!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SmdpSigned2.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    bool? ccRequiredFlag;
    Uint8List? bppEuiccOtpk;
    Uint8List? rpmPending;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x1);
    if (found >= 0) {
      final child = node!.children[found];
      ccRequiredFlag = child.value.isNotEmpty && child.value.first != 0;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F49);
    if (found >= 0) {
      final child = node!.children[found];
      bppEuiccOtpk = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      rpmPending = child.value;
      childIdx = found + 1;
    }
    return SmdpSigned2._(
      transactionId: transactionId,
      ccRequiredFlag: ccRequiredFlag,
      bppEuiccOtpk: bppEuiccOtpk,
      rpmPending: rpmPending,
      asn1: node, // Store original node
    );
  }

  static SmdpSigned2 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return SmdpSigned2.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return SmdpSigned2.fromAsn1(node, tag: tag);
    }
    return SmdpSigned2();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SmdpSigned2(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ccRequiredFlagVal = ccRequiredFlag;
    if (ccRequiredFlagVal != null) {
      final val = ccRequiredFlagVal;
      if (val is Uint8List) {
        buffer.writeln('  ccRequiredFlag: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ccRequiredFlag: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final bppEuiccOtpkVal = bppEuiccOtpk;
    if (bppEuiccOtpkVal != null) {
      final val = bppEuiccOtpkVal;
      if (val is Uint8List) {
        buffer.writeln('  bppEuiccOtpk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  bppEuiccOtpk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmPendingVal = rpmPending;
    if (rpmPendingVal != null) {
      final val = rpmPendingVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmPending: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmPending: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class PrepareDownloadResponse {
  final PrepareDownloadResponseOk? downloadResponseOk;
  final PrepareDownloadResponseError? downloadResponseError;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const PrepareDownloadResponse._({
    this.downloadResponseOk,
    this.downloadResponseError,
    this.asn1,
  });

  const PrepareDownloadResponse({
    PrepareDownloadResponseOk? downloadResponseOk,
    PrepareDownloadResponseError? downloadResponseError,
  }) : this._(
         downloadResponseOk: downloadResponseOk,
         downloadResponseError: downloadResponseError,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF21;
    if (downloadResponseOk != null)
      children.add((downloadResponseOk as dynamic).encode(tag: 0xA0));
    if (downloadResponseError != null)
      children.add((downloadResponseError as dynamic).encode(tag: 0xA1));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory PrepareDownloadResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF21;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    PrepareDownloadResponseOk? downloadResponseOk;
    PrepareDownloadResponseError? downloadResponseError;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          downloadResponseOk = PrepareDownloadResponseOk.decode(
            child,
            tag: 0xA0,
          );
          break;
        case 0xA1:
          downloadResponseError = PrepareDownloadResponseError.decode(
            child,
            tag: 0xA1,
          );
          break;
      }
    }
    return PrepareDownloadResponse._(
      downloadResponseOk: downloadResponseOk,
      downloadResponseError: downloadResponseError,
      asn1: node, // Store original node
    );
  }

  static PrepareDownloadResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return PrepareDownloadResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return PrepareDownloadResponse.fromAsn1(node, tag: tag);
    }
    return PrepareDownloadResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('PrepareDownloadResponse(');
    final downloadResponseOkVal = downloadResponseOk;
    if (downloadResponseOkVal != null) {
      final val = downloadResponseOkVal;
      if (val is Uint8List) {
        buffer.writeln('  downloadResponseOk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  downloadResponseOk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final downloadResponseErrorVal = downloadResponseError;
    if (downloadResponseErrorVal != null) {
      final val = downloadResponseErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  downloadResponseError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  downloadResponseError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class PrepareDownloadResponseOk {
  final EUICCSigned2? euiccSigned2;
  final Uint8List? euiccSignature2;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const PrepareDownloadResponseOk._({
    this.euiccSigned2,
    this.euiccSignature2,
    this.asn1,
  });

  const PrepareDownloadResponseOk({
    EUICCSigned2? euiccSigned2,
    Uint8List? euiccSignature2,
  }) : this._(
         euiccSigned2: euiccSigned2,
         euiccSignature2: euiccSignature2,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (euiccSigned2 != null)
      children.add((euiccSigned2 as dynamic).encode(tag: 0x30));
    if (euiccSignature2 != null)
      children.add(_asn1EncodeRaw(0x5F37, euiccSignature2!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory PrepareDownloadResponseOk.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    EUICCSigned2? euiccSigned2;
    Uint8List? euiccSignature2;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      euiccSigned2 = EUICCSigned2.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      euiccSignature2 = child.value;
      childIdx = found + 1;
    }
    return PrepareDownloadResponseOk._(
      euiccSigned2: euiccSigned2,
      euiccSignature2: euiccSignature2,
      asn1: node, // Store original node
    );
  }

  static PrepareDownloadResponseOk decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return PrepareDownloadResponseOk.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return PrepareDownloadResponseOk.fromAsn1(node, tag: tag);
    }
    return PrepareDownloadResponseOk();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('PrepareDownloadResponseOk(');
    final euiccSigned2Val = euiccSigned2;
    if (euiccSigned2Val != null) {
      final val = euiccSigned2Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccSigned2: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccSigned2: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccSignature2Val = euiccSignature2;
    if (euiccSignature2Val != null) {
      final val = euiccSignature2Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccSignature2: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccSignature2: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EUICCSigned2 {
  final TransactionId? transactionId;
  final Uint8List? euiccOtpk;
  final Octet32? hashCc;
  final VendorSpecificExtension? additionalInformation;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EUICCSigned2._({
    this.transactionId,
    this.euiccOtpk,
    this.hashCc,
    this.additionalInformation,
    this.asn1,
  });

  const EUICCSigned2({
    TransactionId? transactionId,
    Uint8List? euiccOtpk,
    Octet32? hashCc,
    VendorSpecificExtension? additionalInformation,
  }) : this._(
         transactionId: transactionId,
         euiccOtpk: euiccOtpk,
         hashCc: hashCc,
         additionalInformation: additionalInformation,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (euiccOtpk != null) children.add(_asn1EncodeRaw(0x5F49, euiccOtpk!));
    if (hashCc != null) children.add(_asn1EncodeRaw(0x4, hashCc!));
    if (additionalInformation != null) {
      final items = <Uint8List>[];
      for (final v in additionalInformation!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0x30, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EUICCSigned2.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    Uint8List? euiccOtpk;
    Octet32? hashCc;
    VendorSpecificExtension? additionalInformation;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F49);
    if (found >= 0) {
      final child = node!.children[found];
      euiccOtpk = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      hashCc = child.value;
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      final items = <VendorSpecificExtension__item_>[];
      for (final n in child.children) {
        items.add(VendorSpecificExtension__item_.decode(n));
      }
      additionalInformation = items;
      childIdx++;
    }
    return EUICCSigned2._(
      transactionId: transactionId,
      euiccOtpk: euiccOtpk,
      hashCc: hashCc,
      additionalInformation: additionalInformation,
      asn1: node, // Store original node
    );
  }

  static EUICCSigned2 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return EUICCSigned2.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return EUICCSigned2.fromAsn1(node, tag: tag);
    }
    return EUICCSigned2();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EUICCSigned2(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccOtpkVal = euiccOtpk;
    if (euiccOtpkVal != null) {
      final val = euiccOtpkVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccOtpk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccOtpk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final hashCcVal = hashCc;
    if (hashCcVal != null) {
      final val = hashCcVal;
      if (val is Uint8List) {
        buffer.writeln('  hashCc: ${_hex(val)},');
      } else {
        buffer.writeln('  hashCc: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final additionalInformationVal = additionalInformation;
    if (additionalInformationVal != null) {
      final val = additionalInformationVal;
      if (val is Uint8List) {
        buffer.writeln('  additionalInformation: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  additionalInformation: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  additionalInformation: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class PrepareDownloadResponseError {
  final TransactionId? transactionId;
  final DownloadErrorCode? downloadErrorCode;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const PrepareDownloadResponseError._({
    this.transactionId,
    this.downloadErrorCode,
    this.asn1,
  });

  const PrepareDownloadResponseError({
    TransactionId? transactionId,
    DownloadErrorCode? downloadErrorCode,
  }) : this._(
         transactionId: transactionId,
         downloadErrorCode: downloadErrorCode,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (downloadErrorCode != null)
      children.add(
        _asn1EncodeRaw(0x2, _asn1EncodeInt(downloadErrorCode!.value)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory PrepareDownloadResponseError.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    DownloadErrorCode? downloadErrorCode;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x2);
    if (found >= 0) {
      final child = node!.children[found];
      downloadErrorCode = DownloadErrorCodeExt.fromValue(
        _asn1DecodeInt(child.value),
      );
      childIdx = found + 1;
    }
    return PrepareDownloadResponseError._(
      transactionId: transactionId,
      downloadErrorCode: downloadErrorCode,
      asn1: node, // Store original node
    );
  }

  static PrepareDownloadResponseError decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return PrepareDownloadResponseError.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return PrepareDownloadResponseError.fromAsn1(node, tag: tag);
    }
    return PrepareDownloadResponseError();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('PrepareDownloadResponseError(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final downloadErrorCodeVal = downloadErrorCode;
    if (downloadErrorCodeVal != null) {
      final val = downloadErrorCodeVal;
      if (val is Uint8List) {
        buffer.writeln('  downloadErrorCode: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  downloadErrorCode: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class GetEuiccChallengeRequest {
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const GetEuiccChallengeRequest._({this.asn1});
  const GetEuiccChallengeRequest() : this._(asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF2E;
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory GetEuiccChallengeRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF2E;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int childIdx = 0;
    return GetEuiccChallengeRequest._(
      asn1: node, // Store original node
    );
  }

  static GetEuiccChallengeRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return GetEuiccChallengeRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return GetEuiccChallengeRequest.fromAsn1(node, tag: tag);
    }
    return GetEuiccChallengeRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('GetEuiccChallengeRequest(');
    buffer.write(')');
    return buffer.toString();
  }
}

class GetEuiccChallengeResponse {
  final Octet16? euiccChallenge;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const GetEuiccChallengeResponse._({this.euiccChallenge, this.asn1});

  const GetEuiccChallengeResponse({Octet16? euiccChallenge})
    : this._(euiccChallenge: euiccChallenge, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF2E;
    if (euiccChallenge != null)
      children.add(_asn1EncodeRaw(0x80, euiccChallenge!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory GetEuiccChallengeResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF2E;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Octet16? euiccChallenge;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      euiccChallenge = child.value;
      childIdx = found + 1;
    }
    return GetEuiccChallengeResponse._(
      euiccChallenge: euiccChallenge,
      asn1: node, // Store original node
    );
  }

  static GetEuiccChallengeResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return GetEuiccChallengeResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return GetEuiccChallengeResponse.fromAsn1(node, tag: tag);
    }
    return GetEuiccChallengeResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('GetEuiccChallengeResponse(');
    final euiccChallengeVal = euiccChallenge;
    if (euiccChallengeVal != null) {
      final val = euiccChallengeVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccChallenge: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccChallenge: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class GetEuiccInfo1Request {
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const GetEuiccInfo1Request._({this.asn1});
  const GetEuiccInfo1Request() : this._(asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF20;
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory GetEuiccInfo1Request.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF20;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int childIdx = 0;
    return GetEuiccInfo1Request._(
      asn1: node, // Store original node
    );
  }

  static GetEuiccInfo1Request decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return GetEuiccInfo1Request.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return GetEuiccInfo1Request.fromAsn1(node, tag: tag);
    }
    return GetEuiccInfo1Request();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('GetEuiccInfo1Request(');
    buffer.write(')');
    return buffer.toString();
  }
}

class GetEuiccInfo2Request {
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const GetEuiccInfo2Request._({this.asn1});
  const GetEuiccInfo2Request() : this._(asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF22;
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory GetEuiccInfo2Request.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF22;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int childIdx = 0;
    return GetEuiccInfo2Request._(
      asn1: node, // Store original node
    );
  }

  static GetEuiccInfo2Request decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return GetEuiccInfo2Request.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return GetEuiccInfo2Request.fromAsn1(node, tag: tag);
    }
    return GetEuiccInfo2Request();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('GetEuiccInfo2Request(');
    buffer.write(')');
    return buffer.toString();
  }
}

class ListNotificationRequest {
  final NotificationEvent? profileManagementOperation;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ListNotificationRequest._({this.profileManagementOperation, this.asn1});

  const ListNotificationRequest({NotificationEvent? profileManagementOperation})
    : this._(
        profileManagementOperation: profileManagementOperation,
        asn1: null,
      );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF28;
    if (profileManagementOperation != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          _asn1EncodeBits(profileManagementOperation!.value),
        ),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ListNotificationRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF28;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    NotificationEvent? profileManagementOperation;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      profileManagementOperation = NotificationEvent(
        _asn1DecodeBits(child.value),
      );
      childIdx = found + 1;
    }
    return ListNotificationRequest._(
      profileManagementOperation: profileManagementOperation,
      asn1: node, // Store original node
    );
  }

  static ListNotificationRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return ListNotificationRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ListNotificationRequest.fromAsn1(node, tag: tag);
    }
    return ListNotificationRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ListNotificationRequest(');
    final profileManagementOperationVal = profileManagementOperation;
    if (profileManagementOperationVal != null) {
      final val = profileManagementOperationVal;
      if (val is Uint8List) {
        buffer.writeln('  profileManagementOperation: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileManagementOperation: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ListNotificationResponse {
  final List<NotificationMetadata>? notificationMetadataList;
  final ListNotificationResponse_listNotificationsResultError?
  listNotificationsResultError;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ListNotificationResponse._({
    this.notificationMetadataList,
    this.listNotificationsResultError,
    this.asn1,
  });

  const ListNotificationResponse({
    List<NotificationMetadata>? notificationMetadataList,
    ListNotificationResponse_listNotificationsResultError?
    listNotificationsResultError,
  }) : this._(
         notificationMetadataList: notificationMetadataList,
         listNotificationsResultError: listNotificationsResultError,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF28;
    if (notificationMetadataList != null) {
      final items = <Uint8List>[];
      for (final v in notificationMetadataList!) {
        items.add((v as dynamic).encode(tag: 0xBF2F));
      }
      children.add(_asn1EncodeConstructed(0xA0, items));
    }
    if (listNotificationsResultError != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          _asn1EncodeInt(listNotificationsResultError!.value),
        ),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ListNotificationResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF28;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    List<NotificationMetadata>? notificationMetadataList;
    ListNotificationResponse_listNotificationsResultError?
    listNotificationsResultError;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          final items = <NotificationMetadata>[];
          for (final n in child.children) {
            items.add(NotificationMetadata.decode(n));
          }
          notificationMetadataList = items;
          break;
        case 0x81:
          listNotificationsResultError =
              ListNotificationResponse_listNotificationsResultErrorExt.fromValue(
                _asn1DecodeInt(child.value),
              );
          break;
      }
    }
    return ListNotificationResponse._(
      notificationMetadataList: notificationMetadataList,
      listNotificationsResultError: listNotificationsResultError,
      asn1: node, // Store original node
    );
  }

  static ListNotificationResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return ListNotificationResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return ListNotificationResponse.fromAsn1(node, tag: tag);
    }
    return ListNotificationResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ListNotificationResponse(');
    final notificationMetadataListVal = notificationMetadataList;
    if (notificationMetadataListVal != null) {
      final val = notificationMetadataListVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationMetadataList: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  notificationMetadataList: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  notificationMetadataList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final listNotificationsResultErrorVal = listNotificationsResultError;
    if (listNotificationsResultErrorVal != null) {
      final val = listNotificationsResultErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  listNotificationsResultError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  listNotificationsResultError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class NotificationMetadata {
  final int? seqNumber;
  final NotificationEvent? profileManagementOperation;
  final String? notificationAddress;
  final Iccid? iccid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const NotificationMetadata._({
    this.seqNumber,
    this.profileManagementOperation,
    this.notificationAddress,
    this.iccid,
    this.asn1,
  });

  const NotificationMetadata({
    int? seqNumber,
    NotificationEvent? profileManagementOperation,
    String? notificationAddress,
    Iccid? iccid,
  }) : this._(
         seqNumber: seqNumber,
         profileManagementOperation: profileManagementOperation,
         notificationAddress: notificationAddress,
         iccid: iccid,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF2F;
    if (seqNumber != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(seqNumber!)));
    if (profileManagementOperation != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          _asn1EncodeBits(profileManagementOperation!.value),
        ),
      );
    if (notificationAddress != null)
      children.add(
        _asn1EncodeRaw(0xC, Uint8List.fromList(notificationAddress!.codeUnits)),
      );
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory NotificationMetadata.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF2F;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int? seqNumber;
    NotificationEvent? profileManagementOperation;
    String? notificationAddress;
    Iccid? iccid;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      seqNumber = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      profileManagementOperation = NotificationEvent(
        _asn1DecodeBits(child.value),
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xC);
    if (found >= 0) {
      final child = node!.children[found];
      notificationAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    return NotificationMetadata._(
      seqNumber: seqNumber,
      profileManagementOperation: profileManagementOperation,
      notificationAddress: notificationAddress,
      iccid: iccid,
      asn1: node, // Store original node
    );
  }

  static NotificationMetadata decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return NotificationMetadata.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return NotificationMetadata.fromAsn1(node, tag: tag);
    }
    return NotificationMetadata();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('NotificationMetadata(');
    final seqNumberVal = seqNumber;
    if (seqNumberVal != null) {
      final val = seqNumberVal;
      if (val is Uint8List) {
        buffer.writeln('  seqNumber: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  seqNumber: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileManagementOperationVal = profileManagementOperation;
    if (profileManagementOperationVal != null) {
      final val = profileManagementOperationVal;
      if (val is Uint8List) {
        buffer.writeln('  profileManagementOperation: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileManagementOperation: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notificationAddressVal = notificationAddress;
    if (notificationAddressVal != null) {
      final val = notificationAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  notificationAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RetrieveNotificationsListRequest {
  final RetrieveNotificationsListRequest_searchCriteria? searchCriteria;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RetrieveNotificationsListRequest._({this.searchCriteria, this.asn1});

  const RetrieveNotificationsListRequest({
    RetrieveNotificationsListRequest_searchCriteria? searchCriteria,
  }) : this._(searchCriteria: searchCriteria, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF2B;
    if (searchCriteria != null)
      children.add(
        _asn1EncodeConstructed(0xA0, [(searchCriteria as dynamic).encode()]),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RetrieveNotificationsListRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF2B;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    RetrieveNotificationsListRequest_searchCriteria? searchCriteria;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      searchCriteria = RetrieveNotificationsListRequest_searchCriteria.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    return RetrieveNotificationsListRequest._(
      searchCriteria: searchCriteria,
      asn1: node, // Store original node
    );
  }

  static RetrieveNotificationsListRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return RetrieveNotificationsListRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RetrieveNotificationsListRequest.fromAsn1(node, tag: tag);
    }
    return RetrieveNotificationsListRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RetrieveNotificationsListRequest(');
    final searchCriteriaVal = searchCriteria;
    if (searchCriteriaVal != null) {
      final val = searchCriteriaVal;
      if (val is Uint8List) {
        buffer.writeln('  searchCriteria: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  searchCriteria: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RetrieveNotificationsListRequest_searchCriteria {
  final int? seqNumber;
  final NotificationEvent? profileManagementOperation;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RetrieveNotificationsListRequest_searchCriteria._({
    this.seqNumber,
    this.profileManagementOperation,
    this.asn1,
  });

  const RetrieveNotificationsListRequest_searchCriteria({
    int? seqNumber,
    NotificationEvent? profileManagementOperation,
  }) : this._(
         seqNumber: seqNumber,
         profileManagementOperation: profileManagementOperation,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA0;
    if (seqNumber != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(seqNumber!)));
    if (profileManagementOperation != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          _asn1EncodeBits(profileManagementOperation!.value),
        ),
      );
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory RetrieveNotificationsListRequest_searchCriteria.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA0;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int? seqNumber;
    NotificationEvent? profileManagementOperation;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0x80:
          seqNumber = _asn1DecodeInt(child.value);
          break;
        case 0x81:
          profileManagementOperation = NotificationEvent(
            _asn1DecodeBits(child.value),
          );
          break;
      }
    }
    return RetrieveNotificationsListRequest_searchCriteria._(
      seqNumber: seqNumber,
      profileManagementOperation: profileManagementOperation,
      asn1: node, // Store original node
    );
  }

  static RetrieveNotificationsListRequest_searchCriteria decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return RetrieveNotificationsListRequest_searchCriteria.fromAsn1(
        data,
        tag: tag,
      );
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RetrieveNotificationsListRequest_searchCriteria.fromAsn1(
          node,
          tag: tag,
        );
    }
    return RetrieveNotificationsListRequest_searchCriteria();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RetrieveNotificationsListRequest_searchCriteria(');
    final seqNumberVal = seqNumber;
    if (seqNumberVal != null) {
      final val = seqNumberVal;
      if (val is Uint8List) {
        buffer.writeln('  seqNumber: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  seqNumber: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileManagementOperationVal = profileManagementOperation;
    if (profileManagementOperationVal != null) {
      final val = profileManagementOperationVal;
      if (val is Uint8List) {
        buffer.writeln('  profileManagementOperation: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileManagementOperation: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class RetrieveNotificationsListResponse {
  final List<PendingNotification>? notificationList;
  final RetrieveNotificationsListResponse_notificationsListResultError?
  notificationsListResultError;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const RetrieveNotificationsListResponse._({
    this.notificationList,
    this.notificationsListResultError,
    this.asn1,
  });

  const RetrieveNotificationsListResponse({
    List<PendingNotification>? notificationList,
    RetrieveNotificationsListResponse_notificationsListResultError?
    notificationsListResultError,
  }) : this._(
         notificationList: notificationList,
         notificationsListResultError: notificationsListResultError,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF2B;
    if (notificationList != null) {
      final items = <Uint8List>[];
      for (final v in notificationList!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA0, items));
    }
    if (notificationsListResultError != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          _asn1EncodeInt(notificationsListResultError!.value),
        ),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory RetrieveNotificationsListResponse.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xBF2B;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    List<PendingNotification>? notificationList;
    RetrieveNotificationsListResponse_notificationsListResultError?
    notificationsListResultError;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          final items = <PendingNotification>[];
          for (final n in child.children) {
            items.add(PendingNotification.decode(n));
          }
          notificationList = items;
          break;
        case 0x81:
          notificationsListResultError =
              RetrieveNotificationsListResponse_notificationsListResultErrorExt.fromValue(
                _asn1DecodeInt(child.value),
              );
          break;
      }
    }
    return RetrieveNotificationsListResponse._(
      notificationList: notificationList,
      notificationsListResultError: notificationsListResultError,
      asn1: node, // Store original node
    );
  }

  static RetrieveNotificationsListResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return RetrieveNotificationsListResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return RetrieveNotificationsListResponse.fromAsn1(node, tag: tag);
    }
    return RetrieveNotificationsListResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('RetrieveNotificationsListResponse(');
    final notificationListVal = notificationList;
    if (notificationListVal != null) {
      final val = notificationListVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationList: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  notificationList: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  notificationList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notificationsListResultErrorVal = notificationsListResultError;
    if (notificationsListResultErrorVal != null) {
      final val = notificationsListResultErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationsListResultError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  notificationsListResultError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class PendingNotification {
  final ProfileInstallationResult? profileInstallationResult;
  final OtherSignedNotification? otherSignedNotification;
  final LoadRpmPackageResultSigned? loadRpmPackageResultSigned;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const PendingNotification._({
    this.profileInstallationResult,
    this.otherSignedNotification,
    this.loadRpmPackageResultSigned,
    this.asn1,
  });

  const PendingNotification({
    ProfileInstallationResult? profileInstallationResult,
    OtherSignedNotification? otherSignedNotification,
    LoadRpmPackageResultSigned? loadRpmPackageResultSigned,
  }) : this._(
         profileInstallationResult: profileInstallationResult,
         otherSignedNotification: otherSignedNotification,
         loadRpmPackageResultSigned: loadRpmPackageResultSigned,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (profileInstallationResult != null)
      children.add((profileInstallationResult as dynamic).encode(tag: 0xBF37));
    if (otherSignedNotification != null)
      children.add((otherSignedNotification as dynamic).encode(tag: 0x30));
    if (loadRpmPackageResultSigned != null)
      children.add((loadRpmPackageResultSigned as dynamic).encode(tag: 0xA1));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory PendingNotification.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    ProfileInstallationResult? profileInstallationResult;
    OtherSignedNotification? otherSignedNotification;
    LoadRpmPackageResultSigned? loadRpmPackageResultSigned;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0xBF37:
        profileInstallationResult = ProfileInstallationResult.decode(
          node,
          tag: 0xBF37,
        );
        break;
      case 0x30:
        otherSignedNotification = OtherSignedNotification.decode(
          node,
          tag: 0x30,
        );
        break;
      case 0xA1:
        loadRpmPackageResultSigned = LoadRpmPackageResultSigned.decode(
          node,
          tag: 0xA1,
        );
        break;
    }
    return PendingNotification._(
      profileInstallationResult: profileInstallationResult,
      otherSignedNotification: otherSignedNotification,
      loadRpmPackageResultSigned: loadRpmPackageResultSigned,
      asn1: node, // Store original node
    );
  }

  static PendingNotification decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return PendingNotification.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return PendingNotification.fromAsn1(node, tag: tag);
    }
    return PendingNotification();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('PendingNotification(');
    final profileInstallationResultVal = profileInstallationResult;
    if (profileInstallationResultVal != null) {
      final val = profileInstallationResultVal;
      if (val is Uint8List) {
        buffer.writeln('  profileInstallationResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileInstallationResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final otherSignedNotificationVal = otherSignedNotification;
    if (otherSignedNotificationVal != null) {
      final val = otherSignedNotificationVal;
      if (val is Uint8List) {
        buffer.writeln('  otherSignedNotification: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  otherSignedNotification: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final loadRpmPackageResultSignedVal = loadRpmPackageResultSigned;
    if (loadRpmPackageResultSignedVal != null) {
      final val = loadRpmPackageResultSignedVal;
      if (val is Uint8List) {
        buffer.writeln('  loadRpmPackageResultSigned: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  loadRpmPackageResultSigned: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class OtherSignedNotification {
  final NotificationMetadata? tbsOtherNotification;
  final EuiccSign? euiccNotificationSignature;
  final Certificate? euiccCertificate;
  final Certificate? nextCertInChain;
  final CertificateChain? otherCertsInChain;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const OtherSignedNotification._({
    this.tbsOtherNotification,
    this.euiccNotificationSignature,
    this.euiccCertificate,
    this.nextCertInChain,
    this.otherCertsInChain,
    this.asn1,
  });

  const OtherSignedNotification({
    NotificationMetadata? tbsOtherNotification,
    EuiccSign? euiccNotificationSignature,
    Certificate? euiccCertificate,
    Certificate? nextCertInChain,
    CertificateChain? otherCertsInChain,
  }) : this._(
         tbsOtherNotification: tbsOtherNotification,
         euiccNotificationSignature: euiccNotificationSignature,
         euiccCertificate: euiccCertificate,
         nextCertInChain: nextCertInChain,
         otherCertsInChain: otherCertsInChain,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (tbsOtherNotification != null)
      children.add((tbsOtherNotification as dynamic).encode(tag: 0xBF2F));
    if (euiccNotificationSignature != null)
      children.add(_asn1EncodeRaw(0x5F37, euiccNotificationSignature!));
    if (euiccCertificate != null)
      children.add((euiccCertificate as dynamic).encode(tag: 0x30));
    if (nextCertInChain != null)
      children.add((nextCertInChain as dynamic).encode(tag: 0x30));
    if (otherCertsInChain != null) {
      final items = <Uint8List>[];
      for (final v in otherCertsInChain!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA1, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory OtherSignedNotification.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    NotificationMetadata? tbsOtherNotification;
    EuiccSign? euiccNotificationSignature;
    Certificate? euiccCertificate;
    Certificate? nextCertInChain;
    CertificateChain? otherCertsInChain;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xBF2F);
    if (found >= 0) {
      final child = node!.children[found];
      tbsOtherNotification = NotificationMetadata.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      euiccNotificationSignature = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      euiccCertificate = Certificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      nextCertInChain = Certificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Certificate>[];
      for (final n in child.children) {
        items.add(Certificate.decode(n));
      }
      otherCertsInChain = items;
      childIdx = found + 1;
    }
    return OtherSignedNotification._(
      tbsOtherNotification: tbsOtherNotification,
      euiccNotificationSignature: euiccNotificationSignature,
      euiccCertificate: euiccCertificate,
      nextCertInChain: nextCertInChain,
      otherCertsInChain: otherCertsInChain,
      asn1: node, // Store original node
    );
  }

  static OtherSignedNotification decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return OtherSignedNotification.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return OtherSignedNotification.fromAsn1(node, tag: tag);
    }
    return OtherSignedNotification();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('OtherSignedNotification(');
    final tbsOtherNotificationVal = tbsOtherNotification;
    if (tbsOtherNotificationVal != null) {
      final val = tbsOtherNotificationVal;
      if (val is Uint8List) {
        buffer.writeln('  tbsOtherNotification: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tbsOtherNotification: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccNotificationSignatureVal = euiccNotificationSignature;
    if (euiccNotificationSignatureVal != null) {
      final val = euiccNotificationSignatureVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccNotificationSignature: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccNotificationSignature: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCertificateVal = euiccCertificate;
    if (euiccCertificateVal != null) {
      final val = euiccCertificateVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCertificate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccCertificate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final nextCertInChainVal = nextCertInChain;
    if (nextCertInChainVal != null) {
      final val = nextCertInChainVal;
      if (val is Uint8List) {
        buffer.writeln('  nextCertInChain: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  nextCertInChain: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final otherCertsInChainVal = otherCertsInChain;
    if (otherCertsInChainVal != null) {
      final val = otherCertsInChainVal;
      if (val is Uint8List) {
        buffer.writeln('  otherCertsInChain: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  otherCertsInChain: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  otherCertsInChain: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class NotificationSentRequest {
  final int? seqNumber;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const NotificationSentRequest._({this.seqNumber, this.asn1});

  const NotificationSentRequest({int? seqNumber})
    : this._(seqNumber: seqNumber, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF30;
    if (seqNumber != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(seqNumber!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory NotificationSentRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF30;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int? seqNumber;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      seqNumber = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return NotificationSentRequest._(
      seqNumber: seqNumber,
      asn1: node, // Store original node
    );
  }

  static NotificationSentRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return NotificationSentRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return NotificationSentRequest.fromAsn1(node, tag: tag);
    }
    return NotificationSentRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('NotificationSentRequest(');
    final seqNumberVal = seqNumber;
    if (seqNumberVal != null) {
      final val = seqNumberVal;
      if (val is Uint8List) {
        buffer.writeln('  seqNumber: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  seqNumber: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class NotificationSentResponse {
  final NotificationSentResponse_deleteNotificationStatus?
  deleteNotificationStatus;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const NotificationSentResponse._({this.deleteNotificationStatus, this.asn1});

  const NotificationSentResponse({
    NotificationSentResponse_deleteNotificationStatus? deleteNotificationStatus,
  }) : this._(deleteNotificationStatus: deleteNotificationStatus, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF30;
    if (deleteNotificationStatus != null)
      children.add(
        _asn1EncodeRaw(0x80, _asn1EncodeInt(deleteNotificationStatus!.value)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory NotificationSentResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF30;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    NotificationSentResponse_deleteNotificationStatus? deleteNotificationStatus;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      deleteNotificationStatus =
          NotificationSentResponse_deleteNotificationStatusExt.fromValue(
            _asn1DecodeInt(child.value),
          );
      childIdx = found + 1;
    }
    return NotificationSentResponse._(
      deleteNotificationStatus: deleteNotificationStatus,
      asn1: node, // Store original node
    );
  }

  static NotificationSentResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return NotificationSentResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return NotificationSentResponse.fromAsn1(node, tag: tag);
    }
    return NotificationSentResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('NotificationSentResponse(');
    final deleteNotificationStatusVal = deleteNotificationStatus;
    if (deleteNotificationStatusVal != null) {
      final val = deleteNotificationStatusVal;
      if (val is Uint8List) {
        buffer.writeln('  deleteNotificationStatus: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deleteNotificationStatus: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateServerRequest {
  final ServerSigned1? serverSigned1;
  final Uint8List? serverSignature1;
  final SubjectKeyIdentifier? euiccCiPKIdToBeUsed;
  final Certificate? serverCertificate;
  final CtxParams1? ctxParams1;
  final CertificateChain? otherCertsInChain;
  final List<CertificateList>? crlList;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateServerRequest._({
    this.serverSigned1,
    this.serverSignature1,
    this.euiccCiPKIdToBeUsed,
    this.serverCertificate,
    this.ctxParams1,
    this.otherCertsInChain,
    this.crlList,
    this.asn1,
  });

  const AuthenticateServerRequest({
    ServerSigned1? serverSigned1,
    Uint8List? serverSignature1,
    SubjectKeyIdentifier? euiccCiPKIdToBeUsed,
    Certificate? serverCertificate,
    CtxParams1? ctxParams1,
    CertificateChain? otherCertsInChain,
    List<CertificateList>? crlList,
  }) : this._(
         serverSigned1: serverSigned1,
         serverSignature1: serverSignature1,
         euiccCiPKIdToBeUsed: euiccCiPKIdToBeUsed,
         serverCertificate: serverCertificate,
         ctxParams1: ctxParams1,
         otherCertsInChain: otherCertsInChain,
         crlList: crlList,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF38;
    if (serverSigned1 != null)
      children.add((serverSigned1 as dynamic).encode(tag: 0x30));
    if (serverSignature1 != null)
      children.add(_asn1EncodeRaw(0x5F37, serverSignature1!));
    if (euiccCiPKIdToBeUsed != null)
      children.add(_asn1EncodeRaw(0x4, euiccCiPKIdToBeUsed!));
    if (serverCertificate != null)
      children.add((serverCertificate as dynamic).encode(tag: 0x30));
    if (ctxParams1 != null) children.add((ctxParams1 as dynamic).encode());
    if (otherCertsInChain != null) {
      final items = <Uint8List>[];
      for (final v in otherCertsInChain!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA1, items));
    }
    if (crlList != null) {
      final items = <Uint8List>[];
      for (final v in crlList!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA2, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateServerRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF38;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    ServerSigned1? serverSigned1;
    Uint8List? serverSignature1;
    SubjectKeyIdentifier? euiccCiPKIdToBeUsed;
    Certificate? serverCertificate;
    CtxParams1? ctxParams1;
    CertificateChain? otherCertsInChain;
    List<CertificateList>? crlList;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      serverSigned1 = ServerSigned1.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      serverSignature1 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      euiccCiPKIdToBeUsed = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      serverCertificate = Certificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      ctxParams1 = CtxParams1.decode(child, tag: child.tag);
      childIdx++;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Certificate>[];
      for (final n in child.children) {
        items.add(Certificate.decode(n));
      }
      otherCertsInChain = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA2);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <CertificateList>[];
      for (final n in child.children) {
        items.add(CertificateList.decode(n));
      }
      crlList = items;
      childIdx = found + 1;
    }
    return AuthenticateServerRequest._(
      serverSigned1: serverSigned1,
      serverSignature1: serverSignature1,
      euiccCiPKIdToBeUsed: euiccCiPKIdToBeUsed,
      serverCertificate: serverCertificate,
      ctxParams1: ctxParams1,
      otherCertsInChain: otherCertsInChain,
      crlList: crlList,
      asn1: node, // Store original node
    );
  }

  static AuthenticateServerRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return AuthenticateServerRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return AuthenticateServerRequest.fromAsn1(node, tag: tag);
    }
    return AuthenticateServerRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateServerRequest(');
    final serverSigned1Val = serverSigned1;
    if (serverSigned1Val != null) {
      final val = serverSigned1Val;
      if (val is Uint8List) {
        buffer.writeln('  serverSigned1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverSigned1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverSignature1Val = serverSignature1;
    if (serverSignature1Val != null) {
      final val = serverSignature1Val;
      if (val is Uint8List) {
        buffer.writeln('  serverSignature1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverSignature1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdToBeUsedVal = euiccCiPKIdToBeUsed;
    if (euiccCiPKIdToBeUsedVal != null) {
      final val = euiccCiPKIdToBeUsedVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdToBeUsed: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccCiPKIdToBeUsed: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverCertificateVal = serverCertificate;
    if (serverCertificateVal != null) {
      final val = serverCertificateVal;
      if (val is Uint8List) {
        buffer.writeln('  serverCertificate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverCertificate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ctxParams1Val = ctxParams1;
    if (ctxParams1Val != null) {
      final val = ctxParams1Val;
      if (val is Uint8List) {
        buffer.writeln('  ctxParams1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ctxParams1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final otherCertsInChainVal = otherCertsInChain;
    if (otherCertsInChainVal != null) {
      final val = otherCertsInChainVal;
      if (val is Uint8List) {
        buffer.writeln('  otherCertsInChain: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  otherCertsInChain: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  otherCertsInChain: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final crlListVal = crlList;
    if (crlListVal != null) {
      final val = crlListVal;
      if (val is Uint8List) {
        buffer.writeln('  crlList: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  crlList: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  crlList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ServerSigned1 {
  final TransactionId? transactionId;
  final Octet16? euiccChallenge;
  final String? serverAddress;
  final Octet16? serverChallenge;
  final SessionContext? sessionContext;
  final ServerRspCapability? serverRspCapability;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ServerSigned1._({
    this.transactionId,
    this.euiccChallenge,
    this.serverAddress,
    this.serverChallenge,
    this.sessionContext,
    this.serverRspCapability,
    this.asn1,
  });

  const ServerSigned1({
    TransactionId? transactionId,
    Octet16? euiccChallenge,
    String? serverAddress,
    Octet16? serverChallenge,
    SessionContext? sessionContext,
    ServerRspCapability? serverRspCapability,
  }) : this._(
         transactionId: transactionId,
         euiccChallenge: euiccChallenge,
         serverAddress: serverAddress,
         serverChallenge: serverChallenge,
         sessionContext: sessionContext,
         serverRspCapability: serverRspCapability,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (euiccChallenge != null)
      children.add(_asn1EncodeRaw(0x81, euiccChallenge!));
    if (serverAddress != null)
      children.add(
        _asn1EncodeRaw(0x83, Uint8List.fromList(serverAddress!.codeUnits)),
      );
    if (serverChallenge != null)
      children.add(_asn1EncodeRaw(0x84, serverChallenge!));
    if (sessionContext != null)
      children.add((sessionContext as dynamic).encode(tag: 0xA5));
    if (serverRspCapability != null)
      children.add(
        _asn1EncodeRaw(0x86, _asn1EncodeBits(serverRspCapability!.value)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ServerSigned1.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    Octet16? euiccChallenge;
    String? serverAddress;
    Octet16? serverChallenge;
    SessionContext? sessionContext;
    ServerRspCapability? serverRspCapability;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      euiccChallenge = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      serverAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x84);
    if (found >= 0) {
      final child = node!.children[found];
      serverChallenge = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA5);
    if (found >= 0) {
      final child = node!.children[found];
      sessionContext = SessionContext.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x86);
    if (found >= 0) {
      final child = node!.children[found];
      serverRspCapability = ServerRspCapability(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    return ServerSigned1._(
      transactionId: transactionId,
      euiccChallenge: euiccChallenge,
      serverAddress: serverAddress,
      serverChallenge: serverChallenge,
      sessionContext: sessionContext,
      serverRspCapability: serverRspCapability,
      asn1: node, // Store original node
    );
  }

  static ServerSigned1 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return ServerSigned1.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ServerSigned1.fromAsn1(node, tag: tag);
    }
    return ServerSigned1();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ServerSigned1(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccChallengeVal = euiccChallenge;
    if (euiccChallengeVal != null) {
      final val = euiccChallengeVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccChallenge: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccChallenge: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverAddressVal = serverAddress;
    if (serverAddressVal != null) {
      final val = serverAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  serverAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverChallengeVal = serverChallenge;
    if (serverChallengeVal != null) {
      final val = serverChallengeVal;
      if (val is Uint8List) {
        buffer.writeln('  serverChallenge: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverChallenge: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final sessionContextVal = sessionContext;
    if (sessionContextVal != null) {
      final val = sessionContextVal;
      if (val is Uint8List) {
        buffer.writeln('  sessionContext: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  sessionContext: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverRspCapabilityVal = serverRspCapability;
    if (serverRspCapabilityVal != null) {
      final val = serverRspCapabilityVal;
      if (val is Uint8List) {
        buffer.writeln('  serverRspCapability: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverRspCapability: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CtxParams1 {
  final CtxParamsForCommonAuthentication? ctxParamsForCommonAuthentication;
  final CtxParamsForDeviceChange? ctxParamsForDeviceChange;
  final CtxParamsForProfileRecovery? ctxParamsForProfileRecovery;
  final CtxParamsForPushServiceRegistration?
  ctxParamsForPushServiceRegistration;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CtxParams1._({
    this.ctxParamsForCommonAuthentication,
    this.ctxParamsForDeviceChange,
    this.ctxParamsForProfileRecovery,
    this.ctxParamsForPushServiceRegistration,
    this.asn1,
  });

  const CtxParams1({
    CtxParamsForCommonAuthentication? ctxParamsForCommonAuthentication,
    CtxParamsForDeviceChange? ctxParamsForDeviceChange,
    CtxParamsForProfileRecovery? ctxParamsForProfileRecovery,
    CtxParamsForPushServiceRegistration? ctxParamsForPushServiceRegistration,
  }) : this._(
         ctxParamsForCommonAuthentication: ctxParamsForCommonAuthentication,
         ctxParamsForDeviceChange: ctxParamsForDeviceChange,
         ctxParamsForProfileRecovery: ctxParamsForProfileRecovery,
         ctxParamsForPushServiceRegistration:
             ctxParamsForPushServiceRegistration,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (ctxParamsForCommonAuthentication != null)
      children.add(
        (ctxParamsForCommonAuthentication as dynamic).encode(tag: 0xA0),
      );
    if (ctxParamsForDeviceChange != null)
      children.add((ctxParamsForDeviceChange as dynamic).encode(tag: 0xA1));
    if (ctxParamsForProfileRecovery != null)
      children.add((ctxParamsForProfileRecovery as dynamic).encode(tag: 0xA2));
    if (ctxParamsForPushServiceRegistration != null)
      children.add(
        (ctxParamsForPushServiceRegistration as dynamic).encode(tag: 0xA3),
      );
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory CtxParams1.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    CtxParamsForCommonAuthentication? ctxParamsForCommonAuthentication;
    CtxParamsForDeviceChange? ctxParamsForDeviceChange;
    CtxParamsForProfileRecovery? ctxParamsForProfileRecovery;
    CtxParamsForPushServiceRegistration? ctxParamsForPushServiceRegistration;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0xA0:
        ctxParamsForCommonAuthentication =
            CtxParamsForCommonAuthentication.decode(node, tag: 0xA0);
        break;
      case 0xA1:
        ctxParamsForDeviceChange = CtxParamsForDeviceChange.decode(
          node,
          tag: 0xA1,
        );
        break;
      case 0xA2:
        ctxParamsForProfileRecovery = CtxParamsForProfileRecovery.decode(
          node,
          tag: 0xA2,
        );
        break;
      case 0xA3:
        ctxParamsForPushServiceRegistration =
            CtxParamsForPushServiceRegistration.decode(node, tag: 0xA3);
        break;
    }
    return CtxParams1._(
      ctxParamsForCommonAuthentication: ctxParamsForCommonAuthentication,
      ctxParamsForDeviceChange: ctxParamsForDeviceChange,
      ctxParamsForProfileRecovery: ctxParamsForProfileRecovery,
      ctxParamsForPushServiceRegistration: ctxParamsForPushServiceRegistration,
      asn1: node, // Store original node
    );
  }

  static CtxParams1 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return CtxParams1.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return CtxParams1.fromAsn1(node, tag: tag);
    }
    return CtxParams1();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CtxParams1(');
    final ctxParamsForCommonAuthenticationVal =
        ctxParamsForCommonAuthentication;
    if (ctxParamsForCommonAuthenticationVal != null) {
      final val = ctxParamsForCommonAuthenticationVal;
      if (val is Uint8List) {
        buffer.writeln('  ctxParamsForCommonAuthentication: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ctxParamsForCommonAuthentication: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ctxParamsForDeviceChangeVal = ctxParamsForDeviceChange;
    if (ctxParamsForDeviceChangeVal != null) {
      final val = ctxParamsForDeviceChangeVal;
      if (val is Uint8List) {
        buffer.writeln('  ctxParamsForDeviceChange: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ctxParamsForDeviceChange: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ctxParamsForProfileRecoveryVal = ctxParamsForProfileRecovery;
    if (ctxParamsForProfileRecoveryVal != null) {
      final val = ctxParamsForProfileRecoveryVal;
      if (val is Uint8List) {
        buffer.writeln('  ctxParamsForProfileRecovery: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ctxParamsForProfileRecovery: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ctxParamsForPushServiceRegistrationVal =
        ctxParamsForPushServiceRegistration;
    if (ctxParamsForPushServiceRegistrationVal != null) {
      final val = ctxParamsForPushServiceRegistrationVal;
      if (val is Uint8List) {
        buffer.writeln('  ctxParamsForPushServiceRegistration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ctxParamsForPushServiceRegistration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CtxParamsForCommonAuthentication {
  final String? matchingId;
  final DeviceInfo? deviceInfo;
  final OperationType? operationType;
  final Iccid? iccid;
  final MatchingIdSource? matchingIdSource;
  final VendorSpecificExtension? vendorSpecificExtension;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CtxParamsForCommonAuthentication._({
    this.matchingId,
    this.deviceInfo,
    this.operationType,
    this.iccid,
    this.matchingIdSource,
    this.vendorSpecificExtension,
    this.asn1,
  });

  const CtxParamsForCommonAuthentication({
    String? matchingId,
    DeviceInfo? deviceInfo,
    OperationType? operationType,
    Iccid? iccid,
    MatchingIdSource? matchingIdSource,
    VendorSpecificExtension? vendorSpecificExtension,
  }) : this._(
         matchingId: matchingId,
         deviceInfo: deviceInfo,
         operationType: operationType,
         iccid: iccid,
         matchingIdSource: matchingIdSource,
         vendorSpecificExtension: vendorSpecificExtension,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (matchingId != null)
      children.add(
        _asn1EncodeRaw(0x80, Uint8List.fromList(matchingId!.codeUnits)),
      );
    if (deviceInfo != null)
      children.add((deviceInfo as dynamic).encode(tag: 0xA1));
    if (operationType != null)
      children.add(_asn1EncodeRaw(0x82, _asn1EncodeBits(operationType!.value)));
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (matchingIdSource != null)
      children.add(
        _asn1EncodeConstructed(0xA3, [(matchingIdSource as dynamic).encode()]),
      );
    if (vendorSpecificExtension != null) {
      final items = <Uint8List>[];
      for (final v in vendorSpecificExtension!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA4, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CtxParamsForCommonAuthentication.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    String? matchingId;
    DeviceInfo? deviceInfo;
    OperationType? operationType;
    Iccid? iccid;
    MatchingIdSource? matchingIdSource;
    VendorSpecificExtension? vendorSpecificExtension;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      matchingId = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      deviceInfo = DeviceInfo.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      operationType = OperationType(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA3);
    if (found >= 0) {
      final child = node!.children[found];
      matchingIdSource = MatchingIdSource.decode(child.children.first);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA4);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <VendorSpecificExtension__item_>[];
      for (final n in child.children) {
        items.add(VendorSpecificExtension__item_.decode(n));
      }
      vendorSpecificExtension = items;
      childIdx = found + 1;
    }
    return CtxParamsForCommonAuthentication._(
      matchingId: matchingId,
      deviceInfo: deviceInfo,
      operationType: operationType,
      iccid: iccid,
      matchingIdSource: matchingIdSource,
      vendorSpecificExtension: vendorSpecificExtension,
      asn1: node, // Store original node
    );
  }

  static CtxParamsForCommonAuthentication decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return CtxParamsForCommonAuthentication.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return CtxParamsForCommonAuthentication.fromAsn1(node, tag: tag);
    }
    return CtxParamsForCommonAuthentication();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CtxParamsForCommonAuthentication(');
    final matchingIdVal = matchingId;
    if (matchingIdVal != null) {
      final val = matchingIdVal;
      if (val is Uint8List) {
        buffer.writeln('  matchingId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  matchingId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deviceInfoVal = deviceInfo;
    if (deviceInfoVal != null) {
      final val = deviceInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceInfo: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final operationTypeVal = operationType;
    if (operationTypeVal != null) {
      final val = operationTypeVal;
      if (val is Uint8List) {
        buffer.writeln('  operationType: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  operationType: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final matchingIdSourceVal = matchingIdSource;
    if (matchingIdSourceVal != null) {
      final val = matchingIdSourceVal;
      if (val is Uint8List) {
        buffer.writeln('  matchingIdSource: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  matchingIdSource: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final vendorSpecificExtensionVal = vendorSpecificExtension;
    if (vendorSpecificExtensionVal != null) {
      final val = vendorSpecificExtensionVal;
      if (val is Uint8List) {
        buffer.writeln('  vendorSpecificExtension: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  vendorSpecificExtension: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  vendorSpecificExtension: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CtxParamsForDeviceChange {
  final Iccid? iccid;
  final DeviceInfo? deviceInfo;
  final Octet16? targetEidValue;
  final Octet4? targetTacValue;
  final VendorSpecificExtension? vendorSpecificExtension;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CtxParamsForDeviceChange._({
    this.iccid,
    this.deviceInfo,
    this.targetEidValue,
    this.targetTacValue,
    this.vendorSpecificExtension,
    this.asn1,
  });

  const CtxParamsForDeviceChange({
    Iccid? iccid,
    DeviceInfo? deviceInfo,
    Octet16? targetEidValue,
    Octet4? targetTacValue,
    VendorSpecificExtension? vendorSpecificExtension,
  }) : this._(
         iccid: iccid,
         deviceInfo: deviceInfo,
         targetEidValue: targetEidValue,
         targetTacValue: targetTacValue,
         vendorSpecificExtension: vendorSpecificExtension,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (deviceInfo != null)
      children.add((deviceInfo as dynamic).encode(tag: 0xA1));
    if (targetEidValue != null)
      children.add(_asn1EncodeRaw(0x5A, targetEidValue!));
    if (targetTacValue != null)
      children.add(_asn1EncodeRaw(0x82, targetTacValue!));
    if (vendorSpecificExtension != null) {
      final items = <Uint8List>[];
      for (final v in vendorSpecificExtension!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA3, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CtxParamsForDeviceChange.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Iccid? iccid;
    DeviceInfo? deviceInfo;
    Octet16? targetEidValue;
    Octet4? targetTacValue;
    VendorSpecificExtension? vendorSpecificExtension;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      deviceInfo = DeviceInfo.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      targetEidValue = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      targetTacValue = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA3);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <VendorSpecificExtension__item_>[];
      for (final n in child.children) {
        items.add(VendorSpecificExtension__item_.decode(n));
      }
      vendorSpecificExtension = items;
      childIdx = found + 1;
    }
    return CtxParamsForDeviceChange._(
      iccid: iccid,
      deviceInfo: deviceInfo,
      targetEidValue: targetEidValue,
      targetTacValue: targetTacValue,
      vendorSpecificExtension: vendorSpecificExtension,
      asn1: node, // Store original node
    );
  }

  static CtxParamsForDeviceChange decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return CtxParamsForDeviceChange.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return CtxParamsForDeviceChange.fromAsn1(node, tag: tag);
    }
    return CtxParamsForDeviceChange();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CtxParamsForDeviceChange(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final deviceInfoVal = deviceInfo;
    if (deviceInfoVal != null) {
      final val = deviceInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceInfo: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final targetEidValueVal = targetEidValue;
    if (targetEidValueVal != null) {
      final val = targetEidValueVal;
      if (val is Uint8List) {
        buffer.writeln('  targetEidValue: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  targetEidValue: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final targetTacValueVal = targetTacValue;
    if (targetTacValueVal != null) {
      final val = targetTacValueVal;
      if (val is Uint8List) {
        buffer.writeln('  targetTacValue: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  targetTacValue: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final vendorSpecificExtensionVal = vendorSpecificExtension;
    if (vendorSpecificExtensionVal != null) {
      final val = vendorSpecificExtensionVal;
      if (val is Uint8List) {
        buffer.writeln('  vendorSpecificExtension: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  vendorSpecificExtension: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  vendorSpecificExtension: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CtxParamsForProfileRecovery {
  final Iccid? iccid;
  final DeviceInfo? deviceInfo;
  final VendorSpecificExtension? vendorSpecificExtension;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CtxParamsForProfileRecovery._({
    this.iccid,
    this.deviceInfo,
    this.vendorSpecificExtension,
    this.asn1,
  });

  const CtxParamsForProfileRecovery({
    Iccid? iccid,
    DeviceInfo? deviceInfo,
    VendorSpecificExtension? vendorSpecificExtension,
  }) : this._(
         iccid: iccid,
         deviceInfo: deviceInfo,
         vendorSpecificExtension: vendorSpecificExtension,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (deviceInfo != null)
      children.add((deviceInfo as dynamic).encode(tag: 0xA1));
    if (vendorSpecificExtension != null) {
      final items = <Uint8List>[];
      for (final v in vendorSpecificExtension!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA2, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CtxParamsForProfileRecovery.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Iccid? iccid;
    DeviceInfo? deviceInfo;
    VendorSpecificExtension? vendorSpecificExtension;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      deviceInfo = DeviceInfo.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA2);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <VendorSpecificExtension__item_>[];
      for (final n in child.children) {
        items.add(VendorSpecificExtension__item_.decode(n));
      }
      vendorSpecificExtension = items;
      childIdx = found + 1;
    }
    return CtxParamsForProfileRecovery._(
      iccid: iccid,
      deviceInfo: deviceInfo,
      vendorSpecificExtension: vendorSpecificExtension,
      asn1: node, // Store original node
    );
  }

  static CtxParamsForProfileRecovery decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return CtxParamsForProfileRecovery.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return CtxParamsForProfileRecovery.fromAsn1(node, tag: tag);
    }
    return CtxParamsForProfileRecovery();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CtxParamsForProfileRecovery(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final deviceInfoVal = deviceInfo;
    if (deviceInfoVal != null) {
      final val = deviceInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceInfo: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final vendorSpecificExtensionVal = vendorSpecificExtension;
    if (vendorSpecificExtensionVal != null) {
      final val = vendorSpecificExtensionVal;
      if (val is Uint8List) {
        buffer.writeln('  vendorSpecificExtension: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  vendorSpecificExtension: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  vendorSpecificExtension: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CtxParamsForPushServiceRegistration {
  final Uint8List? selectedPushService;
  final String? pushToken;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CtxParamsForPushServiceRegistration._({
    this.selectedPushService,
    this.pushToken,
    this.asn1,
  });

  const CtxParamsForPushServiceRegistration({
    Uint8List? selectedPushService,
    String? pushToken,
  }) : this._(
         selectedPushService: selectedPushService,
         pushToken: pushToken,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (selectedPushService != null)
      children.add(_asn1EncodeRaw(0x80, selectedPushService!));
    if (pushToken != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList(pushToken!.codeUnits)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CtxParamsForPushServiceRegistration.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0x30;
    Uint8List? selectedPushService;
    String? pushToken;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      selectedPushService = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      pushToken = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return CtxParamsForPushServiceRegistration._(
      selectedPushService: selectedPushService,
      pushToken: pushToken,
      asn1: node, // Store original node
    );
  }

  static CtxParamsForPushServiceRegistration decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return CtxParamsForPushServiceRegistration.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return CtxParamsForPushServiceRegistration.fromAsn1(node, tag: tag);
    }
    return CtxParamsForPushServiceRegistration();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CtxParamsForPushServiceRegistration(');
    final selectedPushServiceVal = selectedPushService;
    if (selectedPushServiceVal != null) {
      final val = selectedPushServiceVal;
      if (val is Uint8List) {
        buffer.writeln('  selectedPushService: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  selectedPushService: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final pushTokenVal = pushToken;
    if (pushTokenVal != null) {
      final val = pushTokenVal;
      if (val is Uint8List) {
        buffer.writeln('  pushToken: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  pushToken: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class MatchingIdSource {
  final Uint8List? none;
  final Uint8List? activationCode;
  final Uint8List? smdsOid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const MatchingIdSource._({
    this.none,
    this.activationCode,
    this.smdsOid,
    this.asn1,
  });

  const MatchingIdSource({
    Uint8List? none,
    Uint8List? activationCode,
    Uint8List? smdsOid,
  }) : this._(
         none: none,
         activationCode: activationCode,
         smdsOid: smdsOid,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (none != null) children.add(_asn1EncodeRaw(0x80, none!));
    if (activationCode != null)
      children.add(_asn1EncodeRaw(0x81, activationCode!));
    if (smdsOid != null) children.add(_asn1EncodeRaw(0x82, smdsOid!));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory MatchingIdSource.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    Uint8List? none;
    Uint8List? activationCode;
    Uint8List? smdsOid;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0x80:
        none = node.value;
        break;
      case 0x81:
        activationCode = node.value;
        break;
      case 0x82:
        smdsOid = node.value;
        break;
    }
    return MatchingIdSource._(
      none: none,
      activationCode: activationCode,
      smdsOid: smdsOid,
      asn1: node, // Store original node
    );
  }

  static MatchingIdSource decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return MatchingIdSource.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return MatchingIdSource.fromAsn1(node, tag: tag);
    }
    return MatchingIdSource();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('MatchingIdSource(');
    final noneVal = none;
    if (noneVal != null) {
      final val = noneVal;
      if (val is Uint8List) {
        buffer.writeln('  none: ${_hex(val)},');
      } else {
        buffer.writeln('  none: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final activationCodeVal = activationCode;
    if (activationCodeVal != null) {
      final val = activationCodeVal;
      if (val is Uint8List) {
        buffer.writeln('  activationCode: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  activationCode: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdsOidVal = smdsOid;
    if (smdsOidVal != null) {
      final val = smdsOidVal;
      if (val is Uint8List) {
        buffer.writeln('  smdsOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdsOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SessionContext {
  final VersionType? serverSvn;
  final bool? rlStaplingV3Used;
  final SubjectKeyIdentifier? euiccCiPKIdToBeUsedV3;
  final List<Uint8List>? supportedPushServices;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SessionContext._({
    this.serverSvn,
    this.rlStaplingV3Used,
    this.euiccCiPKIdToBeUsedV3,
    this.supportedPushServices,
    this.asn1,
  });

  const SessionContext({
    VersionType? serverSvn,
    bool? rlStaplingV3Used,
    SubjectKeyIdentifier? euiccCiPKIdToBeUsedV3,
    List<Uint8List>? supportedPushServices,
  }) : this._(
         serverSvn: serverSvn,
         rlStaplingV3Used: rlStaplingV3Used,
         euiccCiPKIdToBeUsedV3: euiccCiPKIdToBeUsedV3,
         supportedPushServices: supportedPushServices,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (serverSvn != null) children.add(_asn1EncodeRaw(0x80, serverSvn!));
    if (rlStaplingV3Used != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          Uint8List.fromList([rlStaplingV3Used! ? 0xFF : 0x00]),
        ),
      );
    if (euiccCiPKIdToBeUsedV3 != null)
      children.add(_asn1EncodeRaw(0x82, euiccCiPKIdToBeUsedV3!));
    if (supportedPushServices != null) {
      final items = <Uint8List>[];
      for (final v in supportedPushServices!) {
        items.add(_asn1EncodeRaw(0x04, v));
      }
      children.add(_asn1EncodeConstructed(0xA3, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SessionContext.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    VersionType? serverSvn;
    bool? rlStaplingV3Used;
    SubjectKeyIdentifier? euiccCiPKIdToBeUsedV3;
    List<Uint8List>? supportedPushServices;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      serverSvn = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      rlStaplingV3Used = child.value.isNotEmpty && child.value.first != 0;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      euiccCiPKIdToBeUsedV3 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA3);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Uint8List>[];
      for (final n in child.children) {
        items.add(n.value);
      }
      supportedPushServices = items;
      childIdx = found + 1;
    }
    return SessionContext._(
      serverSvn: serverSvn,
      rlStaplingV3Used: rlStaplingV3Used,
      euiccCiPKIdToBeUsedV3: euiccCiPKIdToBeUsedV3,
      supportedPushServices: supportedPushServices,
      asn1: node, // Store original node
    );
  }

  static SessionContext decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return SessionContext.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return SessionContext.fromAsn1(node, tag: tag);
    }
    return SessionContext();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SessionContext(');
    final serverSvnVal = serverSvn;
    if (serverSvnVal != null) {
      final val = serverSvnVal;
      if (val is Uint8List) {
        buffer.writeln('  serverSvn: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverSvn: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rlStaplingV3UsedVal = rlStaplingV3Used;
    if (rlStaplingV3UsedVal != null) {
      final val = rlStaplingV3UsedVal;
      if (val is Uint8List) {
        buffer.writeln('  rlStaplingV3Used: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rlStaplingV3Used: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdToBeUsedV3Val = euiccCiPKIdToBeUsedV3;
    if (euiccCiPKIdToBeUsedV3Val != null) {
      final val = euiccCiPKIdToBeUsedV3Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdToBeUsedV3: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccCiPKIdToBeUsedV3: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final supportedPushServicesVal = supportedPushServices;
    if (supportedPushServicesVal != null) {
      final val = supportedPushServicesVal;
      if (val is Uint8List) {
        buffer.writeln('  supportedPushServices: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  supportedPushServices: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  supportedPushServices: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateServerResponse {
  final AuthenticateResponseOk? authenticateResponseOk;
  final AuthenticateResponseError? authenticateResponseError;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateServerResponse._({
    this.authenticateResponseOk,
    this.authenticateResponseError,
    this.asn1,
  });

  const AuthenticateServerResponse({
    AuthenticateResponseOk? authenticateResponseOk,
    AuthenticateResponseError? authenticateResponseError,
  }) : this._(
         authenticateResponseOk: authenticateResponseOk,
         authenticateResponseError: authenticateResponseError,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF38;
    if (authenticateResponseOk != null)
      children.add((authenticateResponseOk as dynamic).encode(tag: 0xA0));
    if (authenticateResponseError != null)
      children.add((authenticateResponseError as dynamic).encode(tag: 0xA1));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateServerResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF38;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    AuthenticateResponseOk? authenticateResponseOk;
    AuthenticateResponseError? authenticateResponseError;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          authenticateResponseOk = AuthenticateResponseOk.decode(
            child,
            tag: 0xA0,
          );
          break;
        case 0xA1:
          authenticateResponseError = AuthenticateResponseError.decode(
            child,
            tag: 0xA1,
          );
          break;
      }
    }
    return AuthenticateServerResponse._(
      authenticateResponseOk: authenticateResponseOk,
      authenticateResponseError: authenticateResponseError,
      asn1: node, // Store original node
    );
  }

  static AuthenticateServerResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return AuthenticateServerResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return AuthenticateServerResponse.fromAsn1(node, tag: tag);
    }
    return AuthenticateServerResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateServerResponse(');
    final authenticateResponseOkVal = authenticateResponseOk;
    if (authenticateResponseOkVal != null) {
      final val = authenticateResponseOkVal;
      if (val is Uint8List) {
        buffer.writeln('  authenticateResponseOk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  authenticateResponseOk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final authenticateResponseErrorVal = authenticateResponseError;
    if (authenticateResponseErrorVal != null) {
      final val = authenticateResponseErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  authenticateResponseError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  authenticateResponseError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateResponseOk {
  final EuiccSigned1? euiccSigned1;
  final Uint8List? euiccSignature1;
  final Certificate? euiccCertificate;
  final Certificate? nextCertInChain;
  final CertificateChain? otherCertsInChain;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateResponseOk._({
    this.euiccSigned1,
    this.euiccSignature1,
    this.euiccCertificate,
    this.nextCertInChain,
    this.otherCertsInChain,
    this.asn1,
  });

  const AuthenticateResponseOk({
    EuiccSigned1? euiccSigned1,
    Uint8List? euiccSignature1,
    Certificate? euiccCertificate,
    Certificate? nextCertInChain,
    CertificateChain? otherCertsInChain,
  }) : this._(
         euiccSigned1: euiccSigned1,
         euiccSignature1: euiccSignature1,
         euiccCertificate: euiccCertificate,
         nextCertInChain: nextCertInChain,
         otherCertsInChain: otherCertsInChain,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (euiccSigned1 != null)
      children.add((euiccSigned1 as dynamic).encode(tag: 0x30));
    if (euiccSignature1 != null)
      children.add(_asn1EncodeRaw(0x5F37, euiccSignature1!));
    if (euiccCertificate != null)
      children.add((euiccCertificate as dynamic).encode(tag: 0x30));
    if (nextCertInChain != null)
      children.add((nextCertInChain as dynamic).encode(tag: 0x30));
    if (otherCertsInChain != null) {
      final items = <Uint8List>[];
      for (final v in otherCertsInChain!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA0, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateResponseOk.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    EuiccSigned1? euiccSigned1;
    Uint8List? euiccSignature1;
    Certificate? euiccCertificate;
    Certificate? nextCertInChain;
    CertificateChain? otherCertsInChain;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      euiccSigned1 = EuiccSigned1.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      euiccSignature1 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      euiccCertificate = Certificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      nextCertInChain = Certificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Certificate>[];
      for (final n in child.children) {
        items.add(Certificate.decode(n));
      }
      otherCertsInChain = items;
      childIdx = found + 1;
    }
    return AuthenticateResponseOk._(
      euiccSigned1: euiccSigned1,
      euiccSignature1: euiccSignature1,
      euiccCertificate: euiccCertificate,
      nextCertInChain: nextCertInChain,
      otherCertsInChain: otherCertsInChain,
      asn1: node, // Store original node
    );
  }

  static AuthenticateResponseOk decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return AuthenticateResponseOk.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return AuthenticateResponseOk.fromAsn1(node, tag: tag);
    }
    return AuthenticateResponseOk();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateResponseOk(');
    final euiccSigned1Val = euiccSigned1;
    if (euiccSigned1Val != null) {
      final val = euiccSigned1Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccSigned1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccSigned1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccSignature1Val = euiccSignature1;
    if (euiccSignature1Val != null) {
      final val = euiccSignature1Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccSignature1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccSignature1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCertificateVal = euiccCertificate;
    if (euiccCertificateVal != null) {
      final val = euiccCertificateVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCertificate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccCertificate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final nextCertInChainVal = nextCertInChain;
    if (nextCertInChainVal != null) {
      final val = nextCertInChainVal;
      if (val is Uint8List) {
        buffer.writeln('  nextCertInChain: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  nextCertInChain: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final otherCertsInChainVal = otherCertsInChain;
    if (otherCertsInChainVal != null) {
      final val = otherCertsInChainVal;
      if (val is Uint8List) {
        buffer.writeln('  otherCertsInChain: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  otherCertsInChain: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  otherCertsInChain: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EuiccSigned1 {
  final TransactionId? transactionId;
  final String? serverAddress;
  final Octet16? serverChallenge;
  final EUICCInfo2? euiccInfo2;
  final CtxParams1? ctxParams1;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EuiccSigned1._({
    this.transactionId,
    this.serverAddress,
    this.serverChallenge,
    this.euiccInfo2,
    this.ctxParams1,
    this.asn1,
  });

  const EuiccSigned1({
    TransactionId? transactionId,
    String? serverAddress,
    Octet16? serverChallenge,
    EUICCInfo2? euiccInfo2,
    CtxParams1? ctxParams1,
  }) : this._(
         transactionId: transactionId,
         serverAddress: serverAddress,
         serverChallenge: serverChallenge,
         euiccInfo2: euiccInfo2,
         ctxParams1: ctxParams1,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (serverAddress != null)
      children.add(
        _asn1EncodeRaw(0x83, Uint8List.fromList(serverAddress!.codeUnits)),
      );
    if (serverChallenge != null)
      children.add(_asn1EncodeRaw(0x84, serverChallenge!));
    if (euiccInfo2 != null)
      children.add((euiccInfo2 as dynamic).encode(tag: 0xBF22));
    if (ctxParams1 != null) children.add((ctxParams1 as dynamic).encode());
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EuiccSigned1.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    String? serverAddress;
    Octet16? serverChallenge;
    EUICCInfo2? euiccInfo2;
    CtxParams1? ctxParams1;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      serverAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x84);
    if (found >= 0) {
      final child = node!.children[found];
      serverChallenge = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF22);
    if (found >= 0) {
      final child = node!.children[found];
      euiccInfo2 = EUICCInfo2.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      ctxParams1 = CtxParams1.decode(child, tag: child.tag);
      childIdx++;
    }
    return EuiccSigned1._(
      transactionId: transactionId,
      serverAddress: serverAddress,
      serverChallenge: serverChallenge,
      euiccInfo2: euiccInfo2,
      ctxParams1: ctxParams1,
      asn1: node, // Store original node
    );
  }

  static EuiccSigned1 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return EuiccSigned1.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return EuiccSigned1.fromAsn1(node, tag: tag);
    }
    return EuiccSigned1();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EuiccSigned1(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverAddressVal = serverAddress;
    if (serverAddressVal != null) {
      final val = serverAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  serverAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverChallengeVal = serverChallenge;
    if (serverChallengeVal != null) {
      final val = serverChallengeVal;
      if (val is Uint8List) {
        buffer.writeln('  serverChallenge: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverChallenge: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccInfo2Val = euiccInfo2;
    if (euiccInfo2Val != null) {
      final val = euiccInfo2Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccInfo2: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccInfo2: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ctxParams1Val = ctxParams1;
    if (ctxParams1Val != null) {
      final val = ctxParams1Val;
      if (val is Uint8List) {
        buffer.writeln('  ctxParams1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ctxParams1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateResponseError {
  final TransactionId? transactionId;
  final AuthenticateErrorCode? authenticateErrorCode;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateResponseError._({
    this.transactionId,
    this.authenticateErrorCode,
    this.asn1,
  });

  const AuthenticateResponseError({
    TransactionId? transactionId,
    AuthenticateErrorCode? authenticateErrorCode,
  }) : this._(
         transactionId: transactionId,
         authenticateErrorCode: authenticateErrorCode,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (authenticateErrorCode != null)
      children.add(
        _asn1EncodeRaw(0x2, _asn1EncodeInt(authenticateErrorCode!.value)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateResponseError.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    AuthenticateErrorCode? authenticateErrorCode;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x2);
    if (found >= 0) {
      final child = node!.children[found];
      authenticateErrorCode = AuthenticateErrorCodeExt.fromValue(
        _asn1DecodeInt(child.value),
      );
      childIdx = found + 1;
    }
    return AuthenticateResponseError._(
      transactionId: transactionId,
      authenticateErrorCode: authenticateErrorCode,
      asn1: node, // Store original node
    );
  }

  static AuthenticateResponseError decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return AuthenticateResponseError.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return AuthenticateResponseError.fromAsn1(node, tag: tag);
    }
    return AuthenticateResponseError();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateResponseError(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final authenticateErrorCodeVal = authenticateErrorCode;
    if (authenticateErrorCodeVal != null) {
      final val = authenticateErrorCodeVal;
      if (val is Uint8List) {
        buffer.writeln('  authenticateErrorCode: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  authenticateErrorCode: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CancelSessionRequest {
  final TransactionId? transactionId;
  final CancelSessionReason? reason;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CancelSessionRequest._({this.transactionId, this.reason, this.asn1});

  const CancelSessionRequest({
    TransactionId? transactionId,
    CancelSessionReason? reason,
  }) : this._(transactionId: transactionId, reason: reason, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF41;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (reason != null)
      children.add(_asn1EncodeRaw(0x81, _asn1EncodeInt(reason!.value)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CancelSessionRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF41;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    TransactionId? transactionId;
    CancelSessionReason? reason;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      reason = CancelSessionReasonExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    return CancelSessionRequest._(
      transactionId: transactionId,
      reason: reason,
      asn1: node, // Store original node
    );
  }

  static CancelSessionRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return CancelSessionRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return CancelSessionRequest.fromAsn1(node, tag: tag);
    }
    return CancelSessionRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CancelSessionRequest(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final reasonVal = reason;
    if (reasonVal != null) {
      final val = reasonVal;
      if (val is Uint8List) {
        buffer.writeln('  reason: ${_hex(val)},');
      } else {
        buffer.writeln('  reason: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CancelSessionResponse {
  final CancelSessionResponseOk? cancelSessionResponseOk;
  final CancelSessionResponse_cancelSessionResponseError?
  cancelSessionResponseError;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CancelSessionResponse._({
    this.cancelSessionResponseOk,
    this.cancelSessionResponseError,
    this.asn1,
  });

  const CancelSessionResponse({
    CancelSessionResponseOk? cancelSessionResponseOk,
    CancelSessionResponse_cancelSessionResponseError?
    cancelSessionResponseError,
  }) : this._(
         cancelSessionResponseOk: cancelSessionResponseOk,
         cancelSessionResponseError: cancelSessionResponseError,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF41;
    if (cancelSessionResponseOk != null)
      children.add((cancelSessionResponseOk as dynamic).encode(tag: 0xA0));
    if (cancelSessionResponseError != null)
      children.add(
        _asn1EncodeRaw(0x81, _asn1EncodeInt(cancelSessionResponseError!.value)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CancelSessionResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF41;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    CancelSessionResponseOk? cancelSessionResponseOk;
    CancelSessionResponse_cancelSessionResponseError?
    cancelSessionResponseError;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          cancelSessionResponseOk = CancelSessionResponseOk.decode(
            child,
            tag: 0xA0,
          );
          break;
        case 0x81:
          cancelSessionResponseError =
              CancelSessionResponse_cancelSessionResponseErrorExt.fromValue(
                _asn1DecodeInt(child.value),
              );
          break;
      }
    }
    return CancelSessionResponse._(
      cancelSessionResponseOk: cancelSessionResponseOk,
      cancelSessionResponseError: cancelSessionResponseError,
      asn1: node, // Store original node
    );
  }

  static CancelSessionResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return CancelSessionResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return CancelSessionResponse.fromAsn1(node, tag: tag);
    }
    return CancelSessionResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CancelSessionResponse(');
    final cancelSessionResponseOkVal = cancelSessionResponseOk;
    if (cancelSessionResponseOkVal != null) {
      final val = cancelSessionResponseOkVal;
      if (val is Uint8List) {
        buffer.writeln('  cancelSessionResponseOk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  cancelSessionResponseOk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final cancelSessionResponseErrorVal = cancelSessionResponseError;
    if (cancelSessionResponseErrorVal != null) {
      final val = cancelSessionResponseErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  cancelSessionResponseError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  cancelSessionResponseError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class CancelSessionResponseOk {
  final EuiccCancelSessionSigned? euiccCancelSessionSigned;
  final Uint8List? euiccCancelSessionSignature;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const CancelSessionResponseOk._({
    this.euiccCancelSessionSigned,
    this.euiccCancelSessionSignature,
    this.asn1,
  });

  const CancelSessionResponseOk({
    EuiccCancelSessionSigned? euiccCancelSessionSigned,
    Uint8List? euiccCancelSessionSignature,
  }) : this._(
         euiccCancelSessionSigned: euiccCancelSessionSigned,
         euiccCancelSessionSignature: euiccCancelSessionSignature,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (euiccCancelSessionSigned != null)
      children.add((euiccCancelSessionSigned as dynamic).encode(tag: 0x30));
    if (euiccCancelSessionSignature != null)
      children.add(_asn1EncodeRaw(0x5F37, euiccCancelSessionSignature!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory CancelSessionResponseOk.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    EuiccCancelSessionSigned? euiccCancelSessionSigned;
    Uint8List? euiccCancelSessionSignature;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      euiccCancelSessionSigned = EuiccCancelSessionSigned.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      euiccCancelSessionSignature = child.value;
      childIdx = found + 1;
    }
    return CancelSessionResponseOk._(
      euiccCancelSessionSigned: euiccCancelSessionSigned,
      euiccCancelSessionSignature: euiccCancelSessionSignature,
      asn1: node, // Store original node
    );
  }

  static CancelSessionResponseOk decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return CancelSessionResponseOk.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return CancelSessionResponseOk.fromAsn1(node, tag: tag);
    }
    return CancelSessionResponseOk();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('CancelSessionResponseOk(');
    final euiccCancelSessionSignedVal = euiccCancelSessionSigned;
    if (euiccCancelSessionSignedVal != null) {
      final val = euiccCancelSessionSignedVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCancelSessionSigned: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccCancelSessionSigned: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCancelSessionSignatureVal = euiccCancelSessionSignature;
    if (euiccCancelSessionSignatureVal != null) {
      final val = euiccCancelSessionSignatureVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCancelSessionSignature: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccCancelSessionSignature: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EuiccCancelSessionSigned {
  final TransactionId? transactionId;
  final Uint8List? smdpOid;
  final CancelSessionReason? reason;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EuiccCancelSessionSigned._({
    this.transactionId,
    this.smdpOid,
    this.reason,
    this.asn1,
  });

  const EuiccCancelSessionSigned({
    TransactionId? transactionId,
    Uint8List? smdpOid,
    CancelSessionReason? reason,
  }) : this._(
         transactionId: transactionId,
         smdpOid: smdpOid,
         reason: reason,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (smdpOid != null) children.add(_asn1EncodeRaw(0x81, smdpOid!));
    if (reason != null)
      children.add(_asn1EncodeRaw(0x82, _asn1EncodeInt(reason!.value)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EuiccCancelSessionSigned.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    Uint8List? smdpOid;
    CancelSessionReason? reason;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      smdpOid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      reason = CancelSessionReasonExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    return EuiccCancelSessionSigned._(
      transactionId: transactionId,
      smdpOid: smdpOid,
      reason: reason,
      asn1: node, // Store original node
    );
  }

  static EuiccCancelSessionSigned decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return EuiccCancelSessionSigned.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return EuiccCancelSessionSigned.fromAsn1(node, tag: tag);
    }
    return EuiccCancelSessionSigned();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EuiccCancelSessionSigned(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpOidVal = smdpOid;
    if (smdpOidVal != null) {
      final val = smdpOidVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final reasonVal = reason;
    if (reasonVal != null) {
      final val = reasonVal;
      if (val is Uint8List) {
        buffer.writeln('  reason: ${_hex(val)},');
      } else {
        buffer.writeln('  reason: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ProfileInfoListRequest {
  final ProfileInfoListRequest_searchCriteria? searchCriteria;
  final Uint8List? tagList;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ProfileInfoListRequest._({
    this.searchCriteria,
    this.tagList,
    this.asn1,
  });

  const ProfileInfoListRequest({
    ProfileInfoListRequest_searchCriteria? searchCriteria,
    Uint8List? tagList,
  }) : this._(searchCriteria: searchCriteria, tagList: tagList, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF2D;
    if (searchCriteria != null)
      children.add(
        _asn1EncodeConstructed(0xA0, [(searchCriteria as dynamic).encode()]),
      );
    if (tagList != null) children.add(_asn1EncodeRaw(0x5C, tagList!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ProfileInfoListRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF2D;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    ProfileInfoListRequest_searchCriteria? searchCriteria;
    Uint8List? tagList;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      searchCriteria = ProfileInfoListRequest_searchCriteria.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5C);
    if (found >= 0) {
      final child = node!.children[found];
      tagList = child.value;
      childIdx = found + 1;
    }
    return ProfileInfoListRequest._(
      searchCriteria: searchCriteria,
      tagList: tagList,
      asn1: node, // Store original node
    );
  }

  static ProfileInfoListRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return ProfileInfoListRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ProfileInfoListRequest.fromAsn1(node, tag: tag);
    }
    return ProfileInfoListRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ProfileInfoListRequest(');
    final searchCriteriaVal = searchCriteria;
    if (searchCriteriaVal != null) {
      final val = searchCriteriaVal;
      if (val is Uint8List) {
        buffer.writeln('  searchCriteria: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  searchCriteria: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final tagListVal = tagList;
    if (tagListVal != null) {
      final val = tagListVal;
      if (val is Uint8List) {
        buffer.writeln('  tagList: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tagList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ProfileInfoListRequest_searchCriteria {
  final OctetTo16? isdpAid;
  final Iccid? iccid;
  final ProfileClass? profileClass;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ProfileInfoListRequest_searchCriteria._({
    this.isdpAid,
    this.iccid,
    this.profileClass,
    this.asn1,
  });

  const ProfileInfoListRequest_searchCriteria({
    OctetTo16? isdpAid,
    Iccid? iccid,
    ProfileClass? profileClass,
  }) : this._(
         isdpAid: isdpAid,
         iccid: iccid,
         profileClass: profileClass,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA0;
    if (isdpAid != null) children.add(_asn1EncodeRaw(0x4F, isdpAid!));
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (profileClass != null)
      children.add(_asn1EncodeRaw(0x95, _asn1EncodeInt(profileClass!.value)));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory ProfileInfoListRequest_searchCriteria.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA0;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    OctetTo16? isdpAid;
    Iccid? iccid;
    ProfileClass? profileClass;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0x4F:
          isdpAid = child.value;
          break;
        case 0x5A:
          iccid = child.value;
          break;
        case 0x95:
          profileClass = ProfileClassExt.fromValue(_asn1DecodeInt(child.value));
          break;
      }
    }
    return ProfileInfoListRequest_searchCriteria._(
      isdpAid: isdpAid,
      iccid: iccid,
      profileClass: profileClass,
      asn1: node, // Store original node
    );
  }

  static ProfileInfoListRequest_searchCriteria decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return ProfileInfoListRequest_searchCriteria.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return ProfileInfoListRequest_searchCriteria.fromAsn1(node, tag: tag);
    }
    return ProfileInfoListRequest_searchCriteria();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ProfileInfoListRequest_searchCriteria(');
    final isdpAidVal = isdpAid;
    if (isdpAidVal != null) {
      final val = isdpAidVal;
      if (val is Uint8List) {
        buffer.writeln('  isdpAid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  isdpAid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final profileClassVal = profileClass;
    if (profileClassVal != null) {
      final val = profileClassVal;
      if (val is Uint8List) {
        buffer.writeln('  profileClass: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileClass: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ListProfileInfo {
  final ListProfileInfo_searchCriteria? searchCriteria;
  final Uint8List? tagList;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ListProfileInfo._({this.searchCriteria, this.tagList, this.asn1});

  const ListProfileInfo({
    ListProfileInfo_searchCriteria? searchCriteria,
    Uint8List? tagList,
  }) : this._(searchCriteria: searchCriteria, tagList: tagList, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA5;
    if (searchCriteria != null)
      children.add(
        _asn1EncodeConstructed(0xA0, [(searchCriteria as dynamic).encode()]),
      );
    if (tagList != null) children.add(_asn1EncodeRaw(0x5C, tagList!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ListProfileInfo.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xA5;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    ListProfileInfo_searchCriteria? searchCriteria;
    Uint8List? tagList;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      searchCriteria = ListProfileInfo_searchCriteria.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5C);
    if (found >= 0) {
      final child = node!.children[found];
      tagList = child.value;
      childIdx = found + 1;
    }
    return ListProfileInfo._(
      searchCriteria: searchCriteria,
      tagList: tagList,
      asn1: node, // Store original node
    );
  }

  static ListProfileInfo decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return ListProfileInfo.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ListProfileInfo.fromAsn1(node, tag: tag);
    }
    return ListProfileInfo();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ListProfileInfo(');
    final searchCriteriaVal = searchCriteria;
    if (searchCriteriaVal != null) {
      final val = searchCriteriaVal;
      if (val is Uint8List) {
        buffer.writeln('  searchCriteria: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  searchCriteria: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final tagListVal = tagList;
    if (tagListVal != null) {
      final val = tagListVal;
      if (val is Uint8List) {
        buffer.writeln('  tagList: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tagList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ListProfileInfo_searchCriteria {
  final Iccid? iccid;
  final Uint8List? profileOwnerOid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ListProfileInfo_searchCriteria._({
    this.iccid,
    this.profileOwnerOid,
    this.asn1,
  });

  const ListProfileInfo_searchCriteria({
    Iccid? iccid,
    Uint8List? profileOwnerOid,
  }) : this._(iccid: iccid, profileOwnerOid: profileOwnerOid, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA0;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (profileOwnerOid != null)
      children.add(_asn1EncodeRaw(0x80, profileOwnerOid!));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory ListProfileInfo_searchCriteria.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xA0;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    Uint8List? profileOwnerOid;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0x5A:
          iccid = child.value;
          break;
        case 0x80:
          profileOwnerOid = child.value;
          break;
      }
    }
    return ListProfileInfo_searchCriteria._(
      iccid: iccid,
      profileOwnerOid: profileOwnerOid,
      asn1: node, // Store original node
    );
  }

  static ListProfileInfo_searchCriteria decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return ListProfileInfo_searchCriteria.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return ListProfileInfo_searchCriteria.fromAsn1(node, tag: tag);
    }
    return ListProfileInfo_searchCriteria();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ListProfileInfo_searchCriteria(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final profileOwnerOidVal = profileOwnerOid;
    if (profileOwnerOidVal != null) {
      final val = profileOwnerOidVal;
      if (val is Uint8List) {
        buffer.writeln('  profileOwnerOid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileOwnerOid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ProfileInfoListResponse {
  final List<ProfileInfo>? profileInfoListOk;
  final ProfileInfoListError? profileInfoListError;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ProfileInfoListResponse._({
    this.profileInfoListOk,
    this.profileInfoListError,
    this.asn1,
  });

  const ProfileInfoListResponse({
    List<ProfileInfo>? profileInfoListOk,
    ProfileInfoListError? profileInfoListError,
  }) : this._(
         profileInfoListOk: profileInfoListOk,
         profileInfoListError: profileInfoListError,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF2D;
    if (profileInfoListOk != null) {
      final items = <Uint8List>[];
      for (final v in profileInfoListOk!) {
        items.add((v as dynamic).encode(tag: 0xE3));
      }
      children.add(_asn1EncodeConstructed(0xA0, items));
    }
    if (profileInfoListError != null)
      children.add(
        _asn1EncodeRaw(0x81, _asn1EncodeInt(profileInfoListError!.value)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ProfileInfoListResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF2D;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    List<ProfileInfo>? profileInfoListOk;
    ProfileInfoListError? profileInfoListError;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          final items = <ProfileInfo>[];
          for (final n in child.children) {
            items.add(ProfileInfo.decode(n));
          }
          profileInfoListOk = items;
          break;
        case 0x81:
          profileInfoListError = ProfileInfoListErrorExt.fromValue(
            _asn1DecodeInt(child.value),
          );
          break;
      }
    }
    return ProfileInfoListResponse._(
      profileInfoListOk: profileInfoListOk,
      profileInfoListError: profileInfoListError,
      asn1: node, // Store original node
    );
  }

  static ProfileInfoListResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return ProfileInfoListResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ProfileInfoListResponse.fromAsn1(node, tag: tag);
    }
    return ProfileInfoListResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ProfileInfoListResponse(');
    final profileInfoListOkVal = profileInfoListOk;
    if (profileInfoListOkVal != null) {
      final val = profileInfoListOkVal;
      if (val is Uint8List) {
        buffer.writeln('  profileInfoListOk: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  profileInfoListOk: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  profileInfoListOk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileInfoListErrorVal = profileInfoListError;
    if (profileInfoListErrorVal != null) {
      final val = profileInfoListErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  profileInfoListError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileInfoListError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ProfileInfo {
  final Iccid? iccid;
  final OctetTo16? isdpAid;
  final ProfileState? profileState;
  final String? profileNickname;
  final String? serviceProviderName;
  final String? profileName;
  final IconType? iconType;
  final Uint8List? icon;
  final ProfileClass? profileClass;
  final List<NotificationConfigurationInformation>?
  notificationConfigurationInfo;
  final OperatorId? profileOwner;
  final DpProprietaryData? dpProprietaryData;
  final PprIds? profilePolicyRules;
  final VendorSpecificExtension? serviceSpecificDataStoredInEuicc;
  final RpmConfiguration? rpmConfiguration;
  final String? hriServerAddress;
  final LprConfiguration? lprConfiguration;
  final EnterpriseConfiguration? enterpriseConfiguration;
  final ServiceDescription? serviceDescription;
  final DeviceChangeConfiguration? deviceChangeConfiguration;
  final int? enabledOnEsimPort;
  final int? profileSize;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ProfileInfo._({
    this.iccid,
    this.isdpAid,
    this.profileState,
    this.profileNickname,
    this.serviceProviderName,
    this.profileName,
    this.iconType,
    this.icon,
    this.profileClass,
    this.notificationConfigurationInfo,
    this.profileOwner,
    this.dpProprietaryData,
    this.profilePolicyRules,
    this.serviceSpecificDataStoredInEuicc,
    this.rpmConfiguration,
    this.hriServerAddress,
    this.lprConfiguration,
    this.enterpriseConfiguration,
    this.serviceDescription,
    this.deviceChangeConfiguration,
    this.enabledOnEsimPort,
    this.profileSize,
    this.asn1,
  });

  const ProfileInfo({
    Iccid? iccid,
    OctetTo16? isdpAid,
    ProfileState? profileState,
    String? profileNickname,
    String? serviceProviderName,
    String? profileName,
    IconType? iconType,
    Uint8List? icon,
    ProfileClass? profileClass,
    List<NotificationConfigurationInformation>? notificationConfigurationInfo,
    OperatorId? profileOwner,
    DpProprietaryData? dpProprietaryData,
    PprIds? profilePolicyRules,
    VendorSpecificExtension? serviceSpecificDataStoredInEuicc,
    RpmConfiguration? rpmConfiguration,
    String? hriServerAddress,
    LprConfiguration? lprConfiguration,
    EnterpriseConfiguration? enterpriseConfiguration,
    ServiceDescription? serviceDescription,
    DeviceChangeConfiguration? deviceChangeConfiguration,
    int? enabledOnEsimPort,
    int? profileSize,
  }) : this._(
         iccid: iccid,
         isdpAid: isdpAid,
         profileState: profileState,
         profileNickname: profileNickname,
         serviceProviderName: serviceProviderName,
         profileName: profileName,
         iconType: iconType,
         icon: icon,
         profileClass: profileClass,
         notificationConfigurationInfo: notificationConfigurationInfo,
         profileOwner: profileOwner,
         dpProprietaryData: dpProprietaryData,
         profilePolicyRules: profilePolicyRules,
         serviceSpecificDataStoredInEuicc: serviceSpecificDataStoredInEuicc,
         rpmConfiguration: rpmConfiguration,
         hriServerAddress: hriServerAddress,
         lprConfiguration: lprConfiguration,
         enterpriseConfiguration: enterpriseConfiguration,
         serviceDescription: serviceDescription,
         deviceChangeConfiguration: deviceChangeConfiguration,
         enabledOnEsimPort: enabledOnEsimPort,
         profileSize: profileSize,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xE3;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (isdpAid != null) children.add(_asn1EncodeRaw(0x4F, isdpAid!));
    if (profileState != null)
      children.add(_asn1EncodeRaw(0x9F70, _asn1EncodeInt(profileState!.value)));
    if (profileNickname != null)
      children.add(
        _asn1EncodeRaw(0x90, Uint8List.fromList(profileNickname!.codeUnits)),
      );
    if (serviceProviderName != null)
      children.add(
        _asn1EncodeRaw(
          0x91,
          Uint8List.fromList(serviceProviderName!.codeUnits),
        ),
      );
    if (profileName != null)
      children.add(
        _asn1EncodeRaw(0x92, Uint8List.fromList(profileName!.codeUnits)),
      );
    if (iconType != null)
      children.add(_asn1EncodeRaw(0x93, _asn1EncodeInt(iconType!.value)));
    if (icon != null) children.add(_asn1EncodeRaw(0x94, icon!));
    if (profileClass != null)
      children.add(_asn1EncodeRaw(0x95, _asn1EncodeInt(profileClass!.value)));
    if (notificationConfigurationInfo != null) {
      final items = <Uint8List>[];
      for (final v in notificationConfigurationInfo!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xB6, items));
    }
    if (profileOwner != null)
      children.add((profileOwner as dynamic).encode(tag: 0xB7));
    if (dpProprietaryData != null)
      children.add((dpProprietaryData as dynamic).encode(tag: 0xB8));
    if (profilePolicyRules != null)
      children.add(
        _asn1EncodeRaw(0x99, _asn1EncodeBits(profilePolicyRules!.value)),
      );
    if (serviceSpecificDataStoredInEuicc != null) {
      final items = <Uint8List>[];
      for (final v in serviceSpecificDataStoredInEuicc!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xBF22, items));
    }
    if (rpmConfiguration != null)
      children.add((rpmConfiguration as dynamic).encode(tag: 0xBA));
    if (hriServerAddress != null)
      children.add(
        _asn1EncodeRaw(0x9B, Uint8List.fromList(hriServerAddress!.codeUnits)),
      );
    if (lprConfiguration != null)
      children.add((lprConfiguration as dynamic).encode(tag: 0xBC));
    if (enterpriseConfiguration != null)
      children.add((enterpriseConfiguration as dynamic).encode(tag: 0xBD));
    if (serviceDescription != null)
      children.add(
        _asn1EncodeRaw(0x9F1F, _asn1EncodeBits(serviceDescription!.value)),
      );
    if (deviceChangeConfiguration != null)
      children.add(
        _asn1EncodeConstructed(0xBF20, [
          (deviceChangeConfiguration as dynamic).encode(),
        ]),
      );
    if (enabledOnEsimPort != null)
      children.add(_asn1EncodeRaw(0x9F24, _asn1EncodeInt(enabledOnEsimPort!)));
    if (profileSize != null)
      children.add(_asn1EncodeRaw(0x9F25, _asn1EncodeInt(profileSize!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ProfileInfo.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xE3;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    OctetTo16? isdpAid;
    ProfileState? profileState;
    String? profileNickname;
    String? serviceProviderName;
    String? profileName;
    IconType? iconType;
    Uint8List? icon;
    ProfileClass? profileClass;
    List<NotificationConfigurationInformation>? notificationConfigurationInfo;
    OperatorId? profileOwner;
    DpProprietaryData? dpProprietaryData;
    PprIds? profilePolicyRules;
    VendorSpecificExtension? serviceSpecificDataStoredInEuicc;
    RpmConfiguration? rpmConfiguration;
    String? hriServerAddress;
    LprConfiguration? lprConfiguration;
    EnterpriseConfiguration? enterpriseConfiguration;
    ServiceDescription? serviceDescription;
    DeviceChangeConfiguration? deviceChangeConfiguration;
    int? enabledOnEsimPort;
    int? profileSize;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4F);
    if (found >= 0) {
      final child = node!.children[found];
      isdpAid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9F70);
    if (found >= 0) {
      final child = node!.children[found];
      profileState = ProfileStateExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x90);
    if (found >= 0) {
      final child = node!.children[found];
      profileNickname = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x91);
    if (found >= 0) {
      final child = node!.children[found];
      serviceProviderName = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x92);
    if (found >= 0) {
      final child = node!.children[found];
      profileName = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x93);
    if (found >= 0) {
      final child = node!.children[found];
      iconType = IconTypeExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x94);
    if (found >= 0) {
      final child = node!.children[found];
      icon = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x95);
    if (found >= 0) {
      final child = node!.children[found];
      profileClass = ProfileClassExt.fromValue(_asn1DecodeInt(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB6);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <NotificationConfigurationInformation>[];
      for (final n in child.children) {
        items.add(NotificationConfigurationInformation.decode(n));
      }
      notificationConfigurationInfo = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB7);
    if (found >= 0) {
      final child = node!.children[found];
      profileOwner = OperatorId.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xB8);
    if (found >= 0) {
      final child = node!.children[found];
      dpProprietaryData = DpProprietaryData.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x99);
    if (found >= 0) {
      final child = node!.children[found];
      profilePolicyRules = PprIds(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF22);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <VendorSpecificExtension__item_>[];
      for (final n in child.children) {
        items.add(VendorSpecificExtension__item_.decode(n));
      }
      serviceSpecificDataStoredInEuicc = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBA);
    if (found >= 0) {
      final child = node!.children[found];
      rpmConfiguration = RpmConfiguration.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9B);
    if (found >= 0) {
      final child = node!.children[found];
      hriServerAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBC);
    if (found >= 0) {
      final child = node!.children[found];
      lprConfiguration = LprConfiguration.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBD);
    if (found >= 0) {
      final child = node!.children[found];
      enterpriseConfiguration = EnterpriseConfiguration.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9F1F);
    if (found >= 0) {
      final child = node!.children[found];
      serviceDescription = ServiceDescription(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF20);
    if (found >= 0) {
      final child = node!.children[found];
      deviceChangeConfiguration = DeviceChangeConfiguration.decode(
        child.children.first,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9F24);
    if (found >= 0) {
      final child = node!.children[found];
      enabledOnEsimPort = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x9F25);
    if (found >= 0) {
      final child = node!.children[found];
      profileSize = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return ProfileInfo._(
      iccid: iccid,
      isdpAid: isdpAid,
      profileState: profileState,
      profileNickname: profileNickname,
      serviceProviderName: serviceProviderName,
      profileName: profileName,
      iconType: iconType,
      icon: icon,
      profileClass: profileClass,
      notificationConfigurationInfo: notificationConfigurationInfo,
      profileOwner: profileOwner,
      dpProprietaryData: dpProprietaryData,
      profilePolicyRules: profilePolicyRules,
      serviceSpecificDataStoredInEuicc: serviceSpecificDataStoredInEuicc,
      rpmConfiguration: rpmConfiguration,
      hriServerAddress: hriServerAddress,
      lprConfiguration: lprConfiguration,
      enterpriseConfiguration: enterpriseConfiguration,
      serviceDescription: serviceDescription,
      deviceChangeConfiguration: deviceChangeConfiguration,
      enabledOnEsimPort: enabledOnEsimPort,
      profileSize: profileSize,
      asn1: node, // Store original node
    );
  }

  static ProfileInfo decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return ProfileInfo.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ProfileInfo.fromAsn1(node, tag: tag);
    }
    return ProfileInfo();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ProfileInfo(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final isdpAidVal = isdpAid;
    if (isdpAidVal != null) {
      final val = isdpAidVal;
      if (val is Uint8List) {
        buffer.writeln('  isdpAid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  isdpAid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileStateVal = profileState;
    if (profileStateVal != null) {
      final val = profileStateVal;
      if (val is Uint8List) {
        buffer.writeln('  profileState: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileState: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileNicknameVal = profileNickname;
    if (profileNicknameVal != null) {
      final val = profileNicknameVal;
      if (val is Uint8List) {
        buffer.writeln('  profileNickname: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileNickname: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceProviderNameVal = serviceProviderName;
    if (serviceProviderNameVal != null) {
      final val = serviceProviderNameVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceProviderName: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serviceProviderName: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileNameVal = profileName;
    if (profileNameVal != null) {
      final val = profileNameVal;
      if (val is Uint8List) {
        buffer.writeln('  profileName: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileName: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iconTypeVal = iconType;
    if (iconTypeVal != null) {
      final val = iconTypeVal;
      buffer.writeln('  iconType: $val,');
    }
    final iconVal = icon;
    if (iconVal != null) {
      final val = iconVal;
      buffer.writeln('  icon: ${val.length} bytes,');
    }
    final profileClassVal = profileClass;
    if (profileClassVal != null) {
      final val = profileClassVal;
      if (val is Uint8List) {
        buffer.writeln('  profileClass: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileClass: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final notificationConfigurationInfoVal = notificationConfigurationInfo;
    if (notificationConfigurationInfoVal != null) {
      final val = notificationConfigurationInfoVal;
      if (val is Uint8List) {
        buffer.writeln('  notificationConfigurationInfo: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  notificationConfigurationInfo: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  notificationConfigurationInfo: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileOwnerVal = profileOwner;
    if (profileOwnerVal != null) {
      final val = profileOwnerVal;
      if (val is Uint8List) {
        buffer.writeln('  profileOwner: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileOwner: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final dpProprietaryDataVal = dpProprietaryData;
    if (dpProprietaryDataVal != null) {
      final val = dpProprietaryDataVal;
      if (val is Uint8List) {
        buffer.writeln('  dpProprietaryData: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  dpProprietaryData: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profilePolicyRulesVal = profilePolicyRules;
    if (profilePolicyRulesVal != null) {
      final val = profilePolicyRulesVal;
      if (val is Uint8List) {
        buffer.writeln('  profilePolicyRules: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profilePolicyRules: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceSpecificDataStoredInEuiccVal =
        serviceSpecificDataStoredInEuicc;
    if (serviceSpecificDataStoredInEuiccVal != null) {
      final val = serviceSpecificDataStoredInEuiccVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceSpecificDataStoredInEuicc: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  serviceSpecificDataStoredInEuicc: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  serviceSpecificDataStoredInEuicc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmConfigurationVal = rpmConfiguration;
    if (rpmConfigurationVal != null) {
      final val = rpmConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final hriServerAddressVal = hriServerAddress;
    if (hriServerAddressVal != null) {
      final val = hriServerAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  hriServerAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  hriServerAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final lprConfigurationVal = lprConfiguration;
    if (lprConfigurationVal != null) {
      final val = lprConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  lprConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  lprConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final enterpriseConfigurationVal = enterpriseConfiguration;
    if (enterpriseConfigurationVal != null) {
      final val = enterpriseConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  enterpriseConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enterpriseConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceDescriptionVal = serviceDescription;
    if (serviceDescriptionVal != null) {
      final val = serviceDescriptionVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceDescription: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serviceDescription: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deviceChangeConfigurationVal = deviceChangeConfiguration;
    if (deviceChangeConfigurationVal != null) {
      final val = deviceChangeConfigurationVal;
      if (val is Uint8List) {
        buffer.writeln('  deviceChangeConfiguration: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deviceChangeConfiguration: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final enabledOnEsimPortVal = enabledOnEsimPort;
    if (enabledOnEsimPortVal != null) {
      final val = enabledOnEsimPortVal;
      if (val is Uint8List) {
        buffer.writeln('  enabledOnEsimPort: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enabledOnEsimPort: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileSizeVal = profileSize;
    if (profileSizeVal != null) {
      final val = profileSizeVal;
      if (val is Uint8List) {
        buffer.writeln('  profileSize: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileSize: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EnableProfileRequest {
  final EnableProfileRequest_profileIdentifier? profileIdentifier;
  final bool? refreshFlag;
  final int? targetEsimPort;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EnableProfileRequest._({
    this.profileIdentifier,
    this.refreshFlag,
    this.targetEsimPort,
    this.asn1,
  });

  const EnableProfileRequest({
    EnableProfileRequest_profileIdentifier? profileIdentifier,
    bool? refreshFlag,
    int? targetEsimPort,
  }) : this._(
         profileIdentifier: profileIdentifier,
         refreshFlag: refreshFlag,
         targetEsimPort: targetEsimPort,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF31;
    if (profileIdentifier != null)
      children.add(
        _asn1EncodeConstructed(0xA0, [(profileIdentifier as dynamic).encode()]),
      );
    if (refreshFlag != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList([refreshFlag! ? 0xFF : 0x00])),
      );
    if (targetEsimPort != null)
      children.add(_asn1EncodeRaw(0x82, _asn1EncodeInt(targetEsimPort!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EnableProfileRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF31;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    EnableProfileRequest_profileIdentifier? profileIdentifier;
    bool? refreshFlag;
    int? targetEsimPort;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      profileIdentifier = EnableProfileRequest_profileIdentifier.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      refreshFlag = child.value.isNotEmpty && child.value.first != 0;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      targetEsimPort = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return EnableProfileRequest._(
      profileIdentifier: profileIdentifier,
      refreshFlag: refreshFlag,
      targetEsimPort: targetEsimPort,
      asn1: node, // Store original node
    );
  }

  static EnableProfileRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return EnableProfileRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return EnableProfileRequest.fromAsn1(node, tag: tag);
    }
    return EnableProfileRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EnableProfileRequest(');
    final profileIdentifierVal = profileIdentifier;
    if (profileIdentifierVal != null) {
      final val = profileIdentifierVal;
      if (val is Uint8List) {
        buffer.writeln('  profileIdentifier: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileIdentifier: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final refreshFlagVal = refreshFlag;
    if (refreshFlagVal != null) {
      final val = refreshFlagVal;
      if (val is Uint8List) {
        buffer.writeln('  refreshFlag: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  refreshFlag: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final targetEsimPortVal = targetEsimPort;
    if (targetEsimPortVal != null) {
      final val = targetEsimPortVal;
      if (val is Uint8List) {
        buffer.writeln('  targetEsimPort: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  targetEsimPort: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EnableProfileRequest_profileIdentifier {
  final OctetTo16? isdpAid;
  final Iccid? iccid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EnableProfileRequest_profileIdentifier._({
    this.isdpAid,
    this.iccid,
    this.asn1,
  });

  const EnableProfileRequest_profileIdentifier({
    OctetTo16? isdpAid,
    Iccid? iccid,
  }) : this._(isdpAid: isdpAid, iccid: iccid, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA0;
    if (isdpAid != null) children.add(_asn1EncodeRaw(0x4F, isdpAid!));
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory EnableProfileRequest_profileIdentifier.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA0;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    OctetTo16? isdpAid;
    Iccid? iccid;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0x4F:
          isdpAid = child.value;
          break;
        case 0x5A:
          iccid = child.value;
          break;
      }
    }
    return EnableProfileRequest_profileIdentifier._(
      isdpAid: isdpAid,
      iccid: iccid,
      asn1: node, // Store original node
    );
  }

  static EnableProfileRequest_profileIdentifier decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return EnableProfileRequest_profileIdentifier.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return EnableProfileRequest_profileIdentifier.fromAsn1(node, tag: tag);
    }
    return EnableProfileRequest_profileIdentifier();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EnableProfileRequest_profileIdentifier(');
    final isdpAidVal = isdpAid;
    if (isdpAidVal != null) {
      final val = isdpAidVal;
      if (val is Uint8List) {
        buffer.writeln('  isdpAid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  isdpAid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EnableProfileResponse {
  final EnableProfileResponse_enableResult? enableResult;
  final int? targetEsimPort;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EnableProfileResponse._({
    this.enableResult,
    this.targetEsimPort,
    this.asn1,
  });

  const EnableProfileResponse({
    EnableProfileResponse_enableResult? enableResult,
    int? targetEsimPort,
  }) : this._(
         enableResult: enableResult,
         targetEsimPort: targetEsimPort,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF31;
    if (enableResult != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(enableResult!.value)));
    if (targetEsimPort != null)
      children.add(_asn1EncodeRaw(0x81, _asn1EncodeInt(targetEsimPort!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EnableProfileResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF31;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    EnableProfileResponse_enableResult? enableResult;
    int? targetEsimPort;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      enableResult = EnableProfileResponse_enableResultExt.fromValue(
        _asn1DecodeInt(child.value),
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      targetEsimPort = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return EnableProfileResponse._(
      enableResult: enableResult,
      targetEsimPort: targetEsimPort,
      asn1: node, // Store original node
    );
  }

  static EnableProfileResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return EnableProfileResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return EnableProfileResponse.fromAsn1(node, tag: tag);
    }
    return EnableProfileResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EnableProfileResponse(');
    final enableResultVal = enableResult;
    if (enableResultVal != null) {
      final val = enableResultVal;
      if (val is Uint8List) {
        buffer.writeln('  enableResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  enableResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final targetEsimPortVal = targetEsimPort;
    if (targetEsimPortVal != null) {
      final val = targetEsimPortVal;
      if (val is Uint8List) {
        buffer.writeln('  targetEsimPort: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  targetEsimPort: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DisableProfileRequest {
  final DisableProfileRequest_profileIdentifier? profileIdentifier;
  final bool? refreshFlag;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DisableProfileRequest._({
    this.profileIdentifier,
    this.refreshFlag,
    this.asn1,
  });

  const DisableProfileRequest({
    DisableProfileRequest_profileIdentifier? profileIdentifier,
    bool? refreshFlag,
  }) : this._(
         profileIdentifier: profileIdentifier,
         refreshFlag: refreshFlag,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF32;
    if (profileIdentifier != null)
      children.add(
        _asn1EncodeConstructed(0xA0, [(profileIdentifier as dynamic).encode()]),
      );
    if (refreshFlag != null)
      children.add(
        _asn1EncodeRaw(0x81, Uint8List.fromList([refreshFlag! ? 0xFF : 0x00])),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DisableProfileRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF32;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    DisableProfileRequest_profileIdentifier? profileIdentifier;
    bool? refreshFlag;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      profileIdentifier = DisableProfileRequest_profileIdentifier.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      refreshFlag = child.value.isNotEmpty && child.value.first != 0;
      childIdx = found + 1;
    }
    return DisableProfileRequest._(
      profileIdentifier: profileIdentifier,
      refreshFlag: refreshFlag,
      asn1: node, // Store original node
    );
  }

  static DisableProfileRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return DisableProfileRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return DisableProfileRequest.fromAsn1(node, tag: tag);
    }
    return DisableProfileRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DisableProfileRequest(');
    final profileIdentifierVal = profileIdentifier;
    if (profileIdentifierVal != null) {
      final val = profileIdentifierVal;
      if (val is Uint8List) {
        buffer.writeln('  profileIdentifier: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileIdentifier: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final refreshFlagVal = refreshFlag;
    if (refreshFlagVal != null) {
      final val = refreshFlagVal;
      if (val is Uint8List) {
        buffer.writeln('  refreshFlag: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  refreshFlag: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DisableProfileRequest_profileIdentifier {
  final OctetTo16? isdpAid;
  final Iccid? iccid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DisableProfileRequest_profileIdentifier._({
    this.isdpAid,
    this.iccid,
    this.asn1,
  });

  const DisableProfileRequest_profileIdentifier({
    OctetTo16? isdpAid,
    Iccid? iccid,
  }) : this._(isdpAid: isdpAid, iccid: iccid, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA0;
    if (isdpAid != null) children.add(_asn1EncodeRaw(0x4F, isdpAid!));
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory DisableProfileRequest_profileIdentifier.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA0;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    OctetTo16? isdpAid;
    Iccid? iccid;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0x4F:
          isdpAid = child.value;
          break;
        case 0x5A:
          iccid = child.value;
          break;
      }
    }
    return DisableProfileRequest_profileIdentifier._(
      isdpAid: isdpAid,
      iccid: iccid,
      asn1: node, // Store original node
    );
  }

  static DisableProfileRequest_profileIdentifier decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return DisableProfileRequest_profileIdentifier.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return DisableProfileRequest_profileIdentifier.fromAsn1(node, tag: tag);
    }
    return DisableProfileRequest_profileIdentifier();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DisableProfileRequest_profileIdentifier(');
    final isdpAidVal = isdpAid;
    if (isdpAidVal != null) {
      final val = isdpAidVal;
      if (val is Uint8List) {
        buffer.writeln('  isdpAid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  isdpAid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DisableProfileResponse {
  final DisableProfileResponse_disableResult? disableResult;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DisableProfileResponse._({this.disableResult, this.asn1});

  const DisableProfileResponse({
    DisableProfileResponse_disableResult? disableResult,
  }) : this._(disableResult: disableResult, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF32;
    if (disableResult != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(disableResult!.value)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DisableProfileResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF32;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    DisableProfileResponse_disableResult? disableResult;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      disableResult = DisableProfileResponse_disableResultExt.fromValue(
        _asn1DecodeInt(child.value),
      );
      childIdx = found + 1;
    }
    return DisableProfileResponse._(
      disableResult: disableResult,
      asn1: node, // Store original node
    );
  }

  static DisableProfileResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return DisableProfileResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return DisableProfileResponse.fromAsn1(node, tag: tag);
    }
    return DisableProfileResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DisableProfileResponse(');
    final disableResultVal = disableResult;
    if (disableResultVal != null) {
      final val = disableResultVal;
      if (val is Uint8List) {
        buffer.writeln('  disableResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  disableResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeleteProfileRequest {
  final OctetTo16? isdpAid;
  final Iccid? iccid;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeleteProfileRequest._({this.isdpAid, this.iccid, this.asn1});

  const DeleteProfileRequest({OctetTo16? isdpAid, Iccid? iccid})
    : this._(isdpAid: isdpAid, iccid: iccid, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF33;
    if (isdpAid != null) children.add(_asn1EncodeRaw(0x4F, isdpAid!));
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DeleteProfileRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF33;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    OctetTo16? isdpAid;
    Iccid? iccid;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0x4F:
          isdpAid = child.value;
          break;
        case 0x5A:
          iccid = child.value;
          break;
      }
    }
    return DeleteProfileRequest._(
      isdpAid: isdpAid,
      iccid: iccid,
      asn1: node, // Store original node
    );
  }

  static DeleteProfileRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return DeleteProfileRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return DeleteProfileRequest.fromAsn1(node, tag: tag);
    }
    return DeleteProfileRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeleteProfileRequest(');
    final isdpAidVal = isdpAid;
    if (isdpAidVal != null) {
      final val = isdpAidVal;
      if (val is Uint8List) {
        buffer.writeln('  isdpAid: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  isdpAid: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class DeleteProfileResponse {
  final DeleteProfileResponse_deleteResult? deleteResult;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const DeleteProfileResponse._({this.deleteResult, this.asn1});

  const DeleteProfileResponse({
    DeleteProfileResponse_deleteResult? deleteResult,
  }) : this._(deleteResult: deleteResult, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF33;
    if (deleteResult != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(deleteResult!.value)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory DeleteProfileResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF33;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    DeleteProfileResponse_deleteResult? deleteResult;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      deleteResult = DeleteProfileResponse_deleteResultExt.fromValue(
        _asn1DecodeInt(child.value),
      );
      childIdx = found + 1;
    }
    return DeleteProfileResponse._(
      deleteResult: deleteResult,
      asn1: node, // Store original node
    );
  }

  static DeleteProfileResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return DeleteProfileResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return DeleteProfileResponse.fromAsn1(node, tag: tag);
    }
    return DeleteProfileResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('DeleteProfileResponse(');
    final deleteResultVal = deleteResult;
    if (deleteResultVal != null) {
      final val = deleteResultVal;
      if (val is Uint8List) {
        buffer.writeln('  deleteResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deleteResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EuiccMemoryResetRequest {
  final int? resetOptions;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EuiccMemoryResetRequest._({this.resetOptions, this.asn1});

  const EuiccMemoryResetRequest({int? resetOptions})
    : this._(resetOptions: resetOptions, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF34;
    if (resetOptions != null)
      children.add(_asn1EncodeRaw(0x82, _asn1EncodeInt(resetOptions!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EuiccMemoryResetRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF34;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int? resetOptions;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      resetOptions = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return EuiccMemoryResetRequest._(
      resetOptions: resetOptions,
      asn1: node, // Store original node
    );
  }

  static EuiccMemoryResetRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return EuiccMemoryResetRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return EuiccMemoryResetRequest.fromAsn1(node, tag: tag);
    }
    return EuiccMemoryResetRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EuiccMemoryResetRequest(');
    final resetOptionsVal = resetOptions;
    if (resetOptionsVal != null) {
      final val = resetOptionsVal;
      if (val is Uint8List) {
        buffer.writeln('  resetOptions: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  resetOptions: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class EuiccMemoryResetResponse {
  final EuiccMemoryResetResponse_resetResult? resetResult;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const EuiccMemoryResetResponse._({this.resetResult, this.asn1});

  const EuiccMemoryResetResponse({
    EuiccMemoryResetResponse_resetResult? resetResult,
  }) : this._(resetResult: resetResult, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF34;
    if (resetResult != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(resetResult!.value)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory EuiccMemoryResetResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF34;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    EuiccMemoryResetResponse_resetResult? resetResult;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      resetResult = EuiccMemoryResetResponse_resetResultExt.fromValue(
        _asn1DecodeInt(child.value),
      );
      childIdx = found + 1;
    }
    return EuiccMemoryResetResponse._(
      resetResult: resetResult,
      asn1: node, // Store original node
    );
  }

  static EuiccMemoryResetResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return EuiccMemoryResetResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return EuiccMemoryResetResponse.fromAsn1(node, tag: tag);
    }
    return EuiccMemoryResetResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('EuiccMemoryResetResponse(');
    final resetResultVal = resetResult;
    if (resetResultVal != null) {
      final val = resetResultVal;
      if (val is Uint8List) {
        buffer.writeln('  resetResult: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  resetResult: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class GetEuiccDataRequest {
  final Octet1? tagList;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const GetEuiccDataRequest._({this.tagList, this.asn1});

  const GetEuiccDataRequest({Octet1? tagList})
    : this._(tagList: tagList, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF3E;
    if (tagList != null) children.add(_asn1EncodeRaw(0x5C, tagList!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory GetEuiccDataRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF3E;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Octet1? tagList;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5C);
    if (found >= 0) {
      final child = node!.children[found];
      tagList = child.value;
      childIdx = found + 1;
    }
    return GetEuiccDataRequest._(
      tagList: tagList,
      asn1: node, // Store original node
    );
  }

  static GetEuiccDataRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return GetEuiccDataRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return GetEuiccDataRequest.fromAsn1(node, tag: tag);
    }
    return GetEuiccDataRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('GetEuiccDataRequest(');
    final tagListVal = tagList;
    if (tagListVal != null) {
      final val = tagListVal;
      if (val is Uint8List) {
        buffer.writeln('  tagList: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  tagList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class GetEuiccDataResponse {
  final Octet16? eidValue;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const GetEuiccDataResponse._({this.eidValue, this.asn1});

  const GetEuiccDataResponse({Octet16? eidValue})
    : this._(eidValue: eidValue, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF3E;
    if (eidValue != null) children.add(_asn1EncodeRaw(0x5A, eidValue!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory GetEuiccDataResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF3E;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Octet16? eidValue;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      eidValue = child.value;
      childIdx = found + 1;
    }
    return GetEuiccDataResponse._(
      eidValue: eidValue,
      asn1: node, // Store original node
    );
  }

  static GetEuiccDataResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return GetEuiccDataResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return GetEuiccDataResponse.fromAsn1(node, tag: tag);
    }
    return GetEuiccDataResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('GetEuiccDataResponse(');
    final eidValueVal = eidValue;
    if (eidValueVal != null) {
      final val = eidValueVal;
      if (val is Uint8List) {
        buffer.writeln('  eidValue: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  eidValue: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SetNicknameRequest {
  final Iccid? iccid;
  final String? profileNickname;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SetNicknameRequest._({this.iccid, this.profileNickname, this.asn1});

  const SetNicknameRequest({Iccid? iccid, String? profileNickname})
    : this._(iccid: iccid, profileNickname: profileNickname, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF29;
    if (iccid != null) children.add(_asn1EncodeRaw(0x5A, iccid!));
    if (profileNickname != null)
      children.add(
        _asn1EncodeRaw(0x90, Uint8List.fromList(profileNickname!.codeUnits)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SetNicknameRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF29;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Iccid? iccid;
    String? profileNickname;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x5A);
    if (found >= 0) {
      final child = node!.children[found];
      iccid = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x90);
    if (found >= 0) {
      final child = node!.children[found];
      profileNickname = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return SetNicknameRequest._(
      iccid: iccid,
      profileNickname: profileNickname,
      asn1: node, // Store original node
    );
  }

  static SetNicknameRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return SetNicknameRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return SetNicknameRequest.fromAsn1(node, tag: tag);
    }
    return SetNicknameRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SetNicknameRequest(');
    final iccidVal = iccid;
    if (iccidVal != null) {
      final val = iccidVal;
      if (val is Uint8List) {
        buffer.writeln('  iccid: ${_hex(val)},');
      } else {
        buffer.writeln('  iccid: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final profileNicknameVal = profileNickname;
    if (profileNicknameVal != null) {
      final val = profileNicknameVal;
      if (val is Uint8List) {
        buffer.writeln('  profileNickname: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileNickname: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class GetRatResponse {
  final RulesAuthorisationTable? rat;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const GetRatResponse._({this.rat, this.asn1});

  const GetRatResponse({RulesAuthorisationTable? rat})
    : this._(rat: rat, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF43;
    if (rat != null) {
      final items = <Uint8List>[];
      for (final v in rat!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA0, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory GetRatResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF43;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    RulesAuthorisationTable? rat;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0xA0);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <ProfilePolicyAuthorisationRule>[];
      for (final n in child.children) {
        items.add(ProfilePolicyAuthorisationRule.decode(n));
      }
      rat = items;
      childIdx = found + 1;
    }
    return GetRatResponse._(
      rat: rat,
      asn1: node, // Store original node
    );
  }

  static GetRatResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return GetRatResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return GetRatResponse.fromAsn1(node, tag: tag);
    }
    return GetRatResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('GetRatResponse(');
    final ratVal = rat;
    if (ratVal != null) {
      final val = ratVal;
      if (val is Uint8List) {
        buffer.writeln('  rat: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  rat: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln('  rat: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ProfilePolicyAuthorisationRule {
  final PprIds? pprIds;
  final List<OperatorId>? allowedOperators;
  final int? pprFlags;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ProfilePolicyAuthorisationRule._({
    this.pprIds,
    this.allowedOperators,
    this.pprFlags,
    this.asn1,
  });

  const ProfilePolicyAuthorisationRule({
    PprIds? pprIds,
    List<OperatorId>? allowedOperators,
    int? pprFlags,
  }) : this._(
         pprIds: pprIds,
         allowedOperators: allowedOperators,
         pprFlags: pprFlags,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (pprIds != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeBits(pprIds!.value)));
    if (allowedOperators != null) {
      final items = <Uint8List>[];
      for (final v in allowedOperators!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA1, items));
    }
    if (pprFlags != null)
      children.add(_asn1EncodeRaw(0x82, _asn1EncodeInt(pprFlags!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ProfilePolicyAuthorisationRule.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    PprIds? pprIds;
    List<OperatorId>? allowedOperators;
    int? pprFlags;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      pprIds = PprIds(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <OperatorId>[];
      for (final n in child.children) {
        items.add(OperatorId.decode(n));
      }
      allowedOperators = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      pprFlags = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return ProfilePolicyAuthorisationRule._(
      pprIds: pprIds,
      allowedOperators: allowedOperators,
      pprFlags: pprFlags,
      asn1: node, // Store original node
    );
  }

  static ProfilePolicyAuthorisationRule decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return ProfilePolicyAuthorisationRule.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return ProfilePolicyAuthorisationRule.fromAsn1(node, tag: tag);
    }
    return ProfilePolicyAuthorisationRule();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ProfilePolicyAuthorisationRule(');
    final pprIdsVal = pprIds;
    if (pprIdsVal != null) {
      final val = pprIdsVal;
      if (val is Uint8List) {
        buffer.writeln('  pprIds: ${_hex(val)},');
      } else {
        buffer.writeln('  pprIds: ${val.toString().replaceAll("\n", "\n  ")},');
      }
    }
    final allowedOperatorsVal = allowedOperators;
    if (allowedOperatorsVal != null) {
      final val = allowedOperatorsVal;
      if (val is Uint8List) {
        buffer.writeln('  allowedOperators: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  allowedOperators: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  allowedOperators: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final pprFlagsVal = pprFlags;
    if (pprFlagsVal != null) {
      final val = pprFlagsVal;
      if (val is Uint8List) {
        buffer.writeln('  pprFlags: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  pprFlags: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SmdpSigned3 {
  final TransactionId? transactionId;
  final RpmPackage? rpmPackage;
  final Uint8List? rpmPending;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SmdpSigned3._({
    this.transactionId,
    this.rpmPackage,
    this.rpmPending,
    this.asn1,
  });

  const SmdpSigned3({
    TransactionId? transactionId,
    RpmPackage? rpmPackage,
    Uint8List? rpmPending,
  }) : this._(
         transactionId: transactionId,
         rpmPackage: rpmPackage,
         rpmPending: rpmPending,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (rpmPackage != null) {
      final items = <Uint8List>[];
      for (final v in rpmPackage!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA1, items));
    }
    if (rpmPending != null) children.add(_asn1EncodeRaw(0x82, rpmPending!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SmdpSigned3.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    RpmPackage? rpmPackage;
    Uint8List? rpmPending;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <RpmCommand>[];
      for (final n in child.children) {
        items.add(RpmCommand.decode(n));
      }
      rpmPackage = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      rpmPending = child.value;
      childIdx = found + 1;
    }
    return SmdpSigned3._(
      transactionId: transactionId,
      rpmPackage: rpmPackage,
      rpmPending: rpmPending,
      asn1: node, // Store original node
    );
  }

  static SmdpSigned3 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return SmdpSigned3.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return SmdpSigned3.fromAsn1(node, tag: tag);
    }
    return SmdpSigned3();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SmdpSigned3(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmPackageVal = rpmPackage;
    if (rpmPackageVal != null) {
      final val = rpmPackageVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmPackage: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  rpmPackage: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  rpmPackage: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final rpmPendingVal = rpmPending;
    if (rpmPendingVal != null) {
      final val = rpmPendingVal;
      if (val is Uint8List) {
        buffer.writeln('  rpmPending: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  rpmPending: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SmdpSigned4 {
  final TransactionId? transactionId;
  final bool? ccRequiredFlag;
  final String? activationCodeForProfileRecovery;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SmdpSigned4._({
    this.transactionId,
    this.ccRequiredFlag,
    this.activationCodeForProfileRecovery,
    this.asn1,
  });

  const SmdpSigned4({
    TransactionId? transactionId,
    bool? ccRequiredFlag,
    String? activationCodeForProfileRecovery,
  }) : this._(
         transactionId: transactionId,
         ccRequiredFlag: ccRequiredFlag,
         activationCodeForProfileRecovery: activationCodeForProfileRecovery,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (ccRequiredFlag != null)
      children.add(
        _asn1EncodeRaw(
          0x1,
          Uint8List.fromList([ccRequiredFlag! ? 0xFF : 0x00]),
        ),
      );
    if (activationCodeForProfileRecovery != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          Uint8List.fromList(activationCodeForProfileRecovery!.codeUnits),
        ),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SmdpSigned4.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    bool? ccRequiredFlag;
    String? activationCodeForProfileRecovery;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x1);
    if (found >= 0) {
      final child = node!.children[found];
      ccRequiredFlag = child.value.isNotEmpty && child.value.first != 0;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      activationCodeForProfileRecovery = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    return SmdpSigned4._(
      transactionId: transactionId,
      ccRequiredFlag: ccRequiredFlag,
      activationCodeForProfileRecovery: activationCodeForProfileRecovery,
      asn1: node, // Store original node
    );
  }

  static SmdpSigned4 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return SmdpSigned4.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return SmdpSigned4.fromAsn1(node, tag: tag);
    }
    return SmdpSigned4();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SmdpSigned4(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final ccRequiredFlagVal = ccRequiredFlag;
    if (ccRequiredFlagVal != null) {
      final val = ccRequiredFlagVal;
      if (val is Uint8List) {
        buffer.writeln('  ccRequiredFlag: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  ccRequiredFlag: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final activationCodeForProfileRecoveryVal =
        activationCodeForProfileRecovery;
    if (activationCodeForProfileRecoveryVal != null) {
      final val = activationCodeForProfileRecoveryVal;
      if (val is Uint8List) {
        buffer.writeln('  activationCodeForProfileRecovery: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  activationCodeForProfileRecovery: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SmdpSigned6 {
  final TransactionId? transactionId;
  final SmdpSigned6_requestSpecificData? requestSpecificData;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SmdpSigned6._({
    this.transactionId,
    this.requestSpecificData,
    this.asn1,
  });

  const SmdpSigned6({
    TransactionId? transactionId,
    SmdpSigned6_requestSpecificData? requestSpecificData,
  }) : this._(
         transactionId: transactionId,
         requestSpecificData: requestSpecificData,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (requestSpecificData != null)
      children.add((requestSpecificData as dynamic).encode());
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SmdpSigned6.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    SmdpSigned6_requestSpecificData? requestSpecificData;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    if (childIdx < node!.children.length) {
      final child = node!.children[childIdx];
      requestSpecificData = SmdpSigned6_requestSpecificData.decode(
        child,
        tag: child.tag,
      );
      childIdx++;
    }
    return SmdpSigned6._(
      transactionId: transactionId,
      requestSpecificData: requestSpecificData,
      asn1: node, // Store original node
    );
  }

  static SmdpSigned6 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return SmdpSigned6.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return SmdpSigned6.fromAsn1(node, tag: tag);
    }
    return SmdpSigned6();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SmdpSigned6(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final requestSpecificDataVal = requestSpecificData;
    if (requestSpecificDataVal != null) {
      final val = requestSpecificDataVal;
      if (val is Uint8List) {
        buffer.writeln('  requestSpecificData: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  requestSpecificData: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SmdpSigned6_requestSpecificData {
  final SmdpSigned6_requestSpecificData_retryData? retryData;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SmdpSigned6_requestSpecificData._({this.retryData, this.asn1});

  const SmdpSigned6_requestSpecificData({
    SmdpSigned6_requestSpecificData_retryData? retryData,
  }) : this._(retryData: retryData, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (retryData != null)
      children.add((retryData as dynamic).encode(tag: 0xA0));
    if (tag != null) {
      return _asn1EncodeConstructed(tag, children);
    }
    return children.isNotEmpty ? children.first : Uint8List(0);
  }

  factory SmdpSigned6_requestSpecificData.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    SmdpSigned6_requestSpecificData_retryData? retryData;
    // CHOICE: check node's tag directly
    switch (node!.tag) {
      case 0xA0:
        retryData = SmdpSigned6_requestSpecificData_retryData.decode(
          node,
          tag: 0xA0,
        );
        break;
    }
    return SmdpSigned6_requestSpecificData._(
      retryData: retryData,
      asn1: node, // Store original node
    );
  }

  static SmdpSigned6_requestSpecificData decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return SmdpSigned6_requestSpecificData.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return SmdpSigned6_requestSpecificData.fromAsn1(node, tag: tag);
    }
    return SmdpSigned6_requestSpecificData();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SmdpSigned6_requestSpecificData(');
    final retryDataVal = retryData;
    if (retryDataVal != null) {
      final val = retryDataVal;
      if (val is Uint8List) {
        buffer.writeln('  retryData: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  retryData: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class SmdpSigned6_requestSpecificData_retryData {
  final int? retryDelay;
  final Uint8List? dcSessionId;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const SmdpSigned6_requestSpecificData_retryData._({
    this.retryDelay,
    this.dcSessionId,
    this.asn1,
  });

  const SmdpSigned6_requestSpecificData_retryData({
    int? retryDelay,
    Uint8List? dcSessionId,
  }) : this._(retryDelay: retryDelay, dcSessionId: dcSessionId, asn1: null);

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xA0;
    if (retryDelay != null)
      children.add(_asn1EncodeRaw(0x80, _asn1EncodeInt(retryDelay!)));
    if (dcSessionId != null) children.add(_asn1EncodeRaw(0x81, dcSessionId!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory SmdpSigned6_requestSpecificData_retryData.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0xA0;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    int? retryDelay;
    Uint8List? dcSessionId;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      retryDelay = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      dcSessionId = child.value;
      childIdx = found + 1;
    }
    return SmdpSigned6_requestSpecificData_retryData._(
      retryDelay: retryDelay,
      dcSessionId: dcSessionId,
      asn1: node, // Store original node
    );
  }

  static SmdpSigned6_requestSpecificData_retryData decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return SmdpSigned6_requestSpecificData_retryData.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return SmdpSigned6_requestSpecificData_retryData.fromAsn1(
          node,
          tag: tag,
        );
    }
    return SmdpSigned6_requestSpecificData_retryData();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('SmdpSigned6_requestSpecificData_retryData(');
    final retryDelayVal = retryDelay;
    if (retryDelayVal != null) {
      final val = retryDelayVal;
      if (val is Uint8List) {
        buffer.writeln('  retryDelay: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  retryDelay: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final dcSessionIdVal = dcSessionId;
    if (dcSessionIdVal != null) {
      final val = dcSessionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  dcSessionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  dcSessionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class InitiateAuthenticationRequest {
  final Octet16? euiccChallenge;
  final String? smdpAddress;
  final EUICCInfo1? euiccInfo1;
  final LpaRspCapability? lpaRspCapability;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const InitiateAuthenticationRequest._({
    this.euiccChallenge,
    this.smdpAddress,
    this.euiccInfo1,
    this.lpaRspCapability,
    this.asn1,
  });

  const InitiateAuthenticationRequest({
    Octet16? euiccChallenge,
    String? smdpAddress,
    EUICCInfo1? euiccInfo1,
    LpaRspCapability? lpaRspCapability,
  }) : this._(
         euiccChallenge: euiccChallenge,
         smdpAddress: smdpAddress,
         euiccInfo1: euiccInfo1,
         lpaRspCapability: lpaRspCapability,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF39;
    if (euiccChallenge != null)
      children.add(_asn1EncodeRaw(0x81, euiccChallenge!));
    if (smdpAddress != null)
      children.add(
        _asn1EncodeRaw(0x83, Uint8List.fromList(smdpAddress!.codeUnits)),
      );
    if (euiccInfo1 != null)
      children.add((euiccInfo1 as dynamic).encode(tag: 0xBF20));
    if (lpaRspCapability != null)
      children.add(
        _asn1EncodeRaw(0x85, _asn1EncodeBits(lpaRspCapability!.value)),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory InitiateAuthenticationRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF39;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    Octet16? euiccChallenge;
    String? smdpAddress;
    EUICCInfo1? euiccInfo1;
    LpaRspCapability? lpaRspCapability;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      euiccChallenge = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      smdpAddress = String.fromCharCodes(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF20);
    if (found >= 0) {
      final child = node!.children[found];
      euiccInfo1 = EUICCInfo1.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x85);
    if (found >= 0) {
      final child = node!.children[found];
      lpaRspCapability = LpaRspCapability(_asn1DecodeBits(child.value));
      childIdx = found + 1;
    }
    return InitiateAuthenticationRequest._(
      euiccChallenge: euiccChallenge,
      smdpAddress: smdpAddress,
      euiccInfo1: euiccInfo1,
      lpaRspCapability: lpaRspCapability,
      asn1: node, // Store original node
    );
  }

  static InitiateAuthenticationRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return InitiateAuthenticationRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return InitiateAuthenticationRequest.fromAsn1(node, tag: tag);
    }
    return InitiateAuthenticationRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('InitiateAuthenticationRequest(');
    final euiccChallengeVal = euiccChallenge;
    if (euiccChallengeVal != null) {
      final val = euiccChallengeVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccChallenge: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccChallenge: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpAddressVal = smdpAddress;
    if (smdpAddressVal != null) {
      final val = smdpAddressVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpAddress: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpAddress: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccInfo1Val = euiccInfo1;
    if (euiccInfo1Val != null) {
      final val = euiccInfo1Val;
      if (val is Uint8List) {
        buffer.writeln('  euiccInfo1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccInfo1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final lpaRspCapabilityVal = lpaRspCapability;
    if (lpaRspCapabilityVal != null) {
      final val = lpaRspCapabilityVal;
      if (val is Uint8List) {
        buffer.writeln('  lpaRspCapability: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  lpaRspCapability: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class InitiateAuthenticationResponse {
  final InitiateAuthenticationOkEs9? initiateAuthenticationOk;
  final InitiateAuthenticationResponse_initiateAuthenticationError?
  initiateAuthenticationError;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const InitiateAuthenticationResponse._({
    this.initiateAuthenticationOk,
    this.initiateAuthenticationError,
    this.asn1,
  });

  const InitiateAuthenticationResponse({
    InitiateAuthenticationOkEs9? initiateAuthenticationOk,
    InitiateAuthenticationResponse_initiateAuthenticationError?
    initiateAuthenticationError,
  }) : this._(
         initiateAuthenticationOk: initiateAuthenticationOk,
         initiateAuthenticationError: initiateAuthenticationError,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF39;
    if (initiateAuthenticationOk != null)
      children.add((initiateAuthenticationOk as dynamic).encode(tag: 0xA0));
    if (initiateAuthenticationError != null)
      children.add(
        _asn1EncodeRaw(
          0x81,
          _asn1EncodeInt(initiateAuthenticationError!.value),
        ),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory InitiateAuthenticationResponse.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF39;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    InitiateAuthenticationOkEs9? initiateAuthenticationOk;
    InitiateAuthenticationResponse_initiateAuthenticationError?
    initiateAuthenticationError;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          initiateAuthenticationOk = InitiateAuthenticationOkEs9.decode(
            child,
            tag: 0xA0,
          );
          break;
        case 0x81:
          initiateAuthenticationError =
              InitiateAuthenticationResponse_initiateAuthenticationErrorExt.fromValue(
                _asn1DecodeInt(child.value),
              );
          break;
      }
    }
    return InitiateAuthenticationResponse._(
      initiateAuthenticationOk: initiateAuthenticationOk,
      initiateAuthenticationError: initiateAuthenticationError,
      asn1: node, // Store original node
    );
  }

  static InitiateAuthenticationResponse decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return InitiateAuthenticationResponse.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return InitiateAuthenticationResponse.fromAsn1(node, tag: tag);
    }
    return InitiateAuthenticationResponse();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('InitiateAuthenticationResponse(');
    final initiateAuthenticationOkVal = initiateAuthenticationOk;
    if (initiateAuthenticationOkVal != null) {
      final val = initiateAuthenticationOkVal;
      if (val is Uint8List) {
        buffer.writeln('  initiateAuthenticationOk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  initiateAuthenticationOk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final initiateAuthenticationErrorVal = initiateAuthenticationError;
    if (initiateAuthenticationErrorVal != null) {
      final val = initiateAuthenticationErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  initiateAuthenticationError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  initiateAuthenticationError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class InitiateAuthenticationOkEs9 {
  final TransactionId? transactionId;
  final ServerSigned1? serverSigned1;
  final Uint8List? serverSignature1;
  final SubjectKeyIdentifier? euiccCiPKIdToBeUsed;
  final Certificate? serverCertificate;
  final CertificateChain? otherCertsInChain;
  final List<CertificateList>? crlList;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const InitiateAuthenticationOkEs9._({
    this.transactionId,
    this.serverSigned1,
    this.serverSignature1,
    this.euiccCiPKIdToBeUsed,
    this.serverCertificate,
    this.otherCertsInChain,
    this.crlList,
    this.asn1,
  });

  const InitiateAuthenticationOkEs9({
    TransactionId? transactionId,
    ServerSigned1? serverSigned1,
    Uint8List? serverSignature1,
    SubjectKeyIdentifier? euiccCiPKIdToBeUsed,
    Certificate? serverCertificate,
    CertificateChain? otherCertsInChain,
    List<CertificateList>? crlList,
  }) : this._(
         transactionId: transactionId,
         serverSigned1: serverSigned1,
         serverSignature1: serverSignature1,
         euiccCiPKIdToBeUsed: euiccCiPKIdToBeUsed,
         serverCertificate: serverCertificate,
         otherCertsInChain: otherCertsInChain,
         crlList: crlList,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (serverSigned1 != null)
      children.add((serverSigned1 as dynamic).encode(tag: 0x30));
    if (serverSignature1 != null)
      children.add(_asn1EncodeRaw(0x5F37, serverSignature1!));
    if (euiccCiPKIdToBeUsed != null)
      children.add(_asn1EncodeRaw(0x4, euiccCiPKIdToBeUsed!));
    if (serverCertificate != null)
      children.add((serverCertificate as dynamic).encode(tag: 0x30));
    if (otherCertsInChain != null) {
      final items = <Uint8List>[];
      for (final v in otherCertsInChain!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA1, items));
    }
    if (crlList != null) {
      final items = <Uint8List>[];
      for (final v in crlList!) {
        items.add((v as dynamic).encode(tag: 0x30));
      }
      children.add(_asn1EncodeConstructed(0xA2, items));
    }
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory InitiateAuthenticationOkEs9.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    ServerSigned1? serverSigned1;
    Uint8List? serverSignature1;
    SubjectKeyIdentifier? euiccCiPKIdToBeUsed;
    Certificate? serverCertificate;
    CertificateChain? otherCertsInChain;
    List<CertificateList>? crlList;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      serverSigned1 = ServerSigned1.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      serverSignature1 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x4);
    if (found >= 0) {
      final child = node!.children[found];
      euiccCiPKIdToBeUsed = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      serverCertificate = Certificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <Certificate>[];
      for (final n in child.children) {
        items.add(Certificate.decode(n));
      }
      otherCertsInChain = items;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA2);
    if (found >= 0) {
      final child = node!.children[found];
      final items = <CertificateList>[];
      for (final n in child.children) {
        items.add(CertificateList.decode(n));
      }
      crlList = items;
      childIdx = found + 1;
    }
    return InitiateAuthenticationOkEs9._(
      transactionId: transactionId,
      serverSigned1: serverSigned1,
      serverSignature1: serverSignature1,
      euiccCiPKIdToBeUsed: euiccCiPKIdToBeUsed,
      serverCertificate: serverCertificate,
      otherCertsInChain: otherCertsInChain,
      crlList: crlList,
      asn1: node, // Store original node
    );
  }

  static InitiateAuthenticationOkEs9 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return InitiateAuthenticationOkEs9.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return InitiateAuthenticationOkEs9.fromAsn1(node, tag: tag);
    }
    return InitiateAuthenticationOkEs9();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('InitiateAuthenticationOkEs9(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverSigned1Val = serverSigned1;
    if (serverSigned1Val != null) {
      final val = serverSigned1Val;
      if (val is Uint8List) {
        buffer.writeln('  serverSigned1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverSigned1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverSignature1Val = serverSignature1;
    if (serverSignature1Val != null) {
      final val = serverSignature1Val;
      if (val is Uint8List) {
        buffer.writeln('  serverSignature1: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverSignature1: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final euiccCiPKIdToBeUsedVal = euiccCiPKIdToBeUsed;
    if (euiccCiPKIdToBeUsedVal != null) {
      final val = euiccCiPKIdToBeUsedVal;
      if (val is Uint8List) {
        buffer.writeln('  euiccCiPKIdToBeUsed: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  euiccCiPKIdToBeUsed: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serverCertificateVal = serverCertificate;
    if (serverCertificateVal != null) {
      final val = serverCertificateVal;
      if (val is Uint8List) {
        buffer.writeln('  serverCertificate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serverCertificate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final otherCertsInChainVal = otherCertsInChain;
    if (otherCertsInChainVal != null) {
      final val = otherCertsInChainVal;
      if (val is Uint8List) {
        buffer.writeln('  otherCertsInChain: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  otherCertsInChain: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  otherCertsInChain: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final crlListVal = crlList;
    if (crlListVal != null) {
      final val = crlListVal;
      if (val is Uint8List) {
        buffer.writeln('  crlList: ${_hex(val)},');
      } else if (val is List) {
        buffer.writeln(
          '  crlList: [${val.map((e) => e.toString().replaceAll("\n", "\n  ")).join(", ")}],',
        );
      } else {
        buffer.writeln(
          '  crlList: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateClientRequest {
  final TransactionId? transactionId;
  final AuthenticateServerResponse? authenticateServerResponse;
  final DeleteNotificationForDc? deleteNotificationForDc;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateClientRequest._({
    this.transactionId,
    this.authenticateServerResponse,
    this.deleteNotificationForDc,
    this.asn1,
  });

  const AuthenticateClientRequest({
    TransactionId? transactionId,
    AuthenticateServerResponse? authenticateServerResponse,
    DeleteNotificationForDc? deleteNotificationForDc,
  }) : this._(
         transactionId: transactionId,
         authenticateServerResponse: authenticateServerResponse,
         deleteNotificationForDc: deleteNotificationForDc,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF3B;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (authenticateServerResponse != null)
      children.add((authenticateServerResponse as dynamic).encode(tag: 0xBF38));
    if (deleteNotificationForDc != null)
      children.add((deleteNotificationForDc as dynamic).encode(tag: 0xBF63));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateClientRequest.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF3B;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    TransactionId? transactionId;
    AuthenticateServerResponse? authenticateServerResponse;
    DeleteNotificationForDc? deleteNotificationForDc;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF38);
    if (found >= 0) {
      final child = node!.children[found];
      authenticateServerResponse = AuthenticateServerResponse.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF63);
    if (found >= 0) {
      final child = node!.children[found];
      deleteNotificationForDc = DeleteNotificationForDc.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    return AuthenticateClientRequest._(
      transactionId: transactionId,
      authenticateServerResponse: authenticateServerResponse,
      deleteNotificationForDc: deleteNotificationForDc,
      asn1: node, // Store original node
    );
  }

  static AuthenticateClientRequest decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return AuthenticateClientRequest.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return AuthenticateClientRequest.fromAsn1(node, tag: tag);
    }
    return AuthenticateClientRequest();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateClientRequest(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final authenticateServerResponseVal = authenticateServerResponse;
    if (authenticateServerResponseVal != null) {
      final val = authenticateServerResponseVal;
      if (val is Uint8List) {
        buffer.writeln('  authenticateServerResponse: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  authenticateServerResponse: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final deleteNotificationForDcVal = deleteNotificationForDc;
    if (deleteNotificationForDcVal != null) {
      final val = deleteNotificationForDcVal;
      if (val is Uint8List) {
        buffer.writeln('  deleteNotificationForDc: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  deleteNotificationForDc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateClientResponseEs9 {
  final AuthenticateClientOk? authenticateClientOk;
  final AuthenticateClientResponseEs9_authenticateClientError?
  authenticateClientError;
  final AuthenticateClientOkRpm? authenticateClientOkRpm;
  final AuthenticateClientOkDeviceChange? authenticateClientOkDeviceChange;
  final AuthenticateClientOkDelayedDeviceChange?
  authenticateClientOkDelayedDeviceChange;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateClientResponseEs9._({
    this.authenticateClientOk,
    this.authenticateClientError,
    this.authenticateClientOkRpm,
    this.authenticateClientOkDeviceChange,
    this.authenticateClientOkDelayedDeviceChange,
    this.asn1,
  });

  const AuthenticateClientResponseEs9({
    AuthenticateClientOk? authenticateClientOk,
    AuthenticateClientResponseEs9_authenticateClientError?
    authenticateClientError,
    AuthenticateClientOkRpm? authenticateClientOkRpm,
    AuthenticateClientOkDeviceChange? authenticateClientOkDeviceChange,
    AuthenticateClientOkDelayedDeviceChange?
    authenticateClientOkDelayedDeviceChange,
  }) : this._(
         authenticateClientOk: authenticateClientOk,
         authenticateClientError: authenticateClientError,
         authenticateClientOkRpm: authenticateClientOkRpm,
         authenticateClientOkDeviceChange: authenticateClientOkDeviceChange,
         authenticateClientOkDelayedDeviceChange:
             authenticateClientOkDelayedDeviceChange,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0xBF3B;
    if (authenticateClientOk != null)
      children.add((authenticateClientOk as dynamic).encode(tag: 0xA0));
    if (authenticateClientError != null)
      children.add(
        _asn1EncodeRaw(0x81, _asn1EncodeInt(authenticateClientError!.value)),
      );
    if (authenticateClientOkRpm != null)
      children.add((authenticateClientOkRpm as dynamic).encode(tag: 0xA2));
    if (authenticateClientOkDeviceChange != null)
      children.add(
        (authenticateClientOkDeviceChange as dynamic).encode(tag: 0xA3),
      );
    if (authenticateClientOkDelayedDeviceChange != null)
      children.add(
        (authenticateClientOkDelayedDeviceChange as dynamic).encode(tag: 0xA4),
      );
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateClientResponseEs9.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0xBF3B;
    if (node.tag != (tag ?? defaultTag)) {
      /* tag mismatch */
    }
    AuthenticateClientOk? authenticateClientOk;
    AuthenticateClientResponseEs9_authenticateClientError?
    authenticateClientError;
    AuthenticateClientOkRpm? authenticateClientOkRpm;
    AuthenticateClientOkDeviceChange? authenticateClientOkDeviceChange;
    AuthenticateClientOkDelayedDeviceChange?
    authenticateClientOkDelayedDeviceChange;
    for (final child in node!.children) {
      switch (child.tag) {
        case 0xA0:
          authenticateClientOk = AuthenticateClientOk.decode(child, tag: 0xA0);
          break;
        case 0x81:
          authenticateClientError =
              AuthenticateClientResponseEs9_authenticateClientErrorExt.fromValue(
                _asn1DecodeInt(child.value),
              );
          break;
        case 0xA2:
          authenticateClientOkRpm = AuthenticateClientOkRpm.decode(
            child,
            tag: 0xA2,
          );
          break;
        case 0xA3:
          authenticateClientOkDeviceChange =
              AuthenticateClientOkDeviceChange.decode(child, tag: 0xA3);
          break;
        case 0xA4:
          authenticateClientOkDelayedDeviceChange =
              AuthenticateClientOkDelayedDeviceChange.decode(child, tag: 0xA4);
          break;
      }
    }
    return AuthenticateClientResponseEs9._(
      authenticateClientOk: authenticateClientOk,
      authenticateClientError: authenticateClientError,
      authenticateClientOkRpm: authenticateClientOkRpm,
      authenticateClientOkDeviceChange: authenticateClientOkDeviceChange,
      authenticateClientOkDelayedDeviceChange:
          authenticateClientOkDelayedDeviceChange,
      asn1: node, // Store original node
    );
  }

  static AuthenticateClientResponseEs9 decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return AuthenticateClientResponseEs9.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return AuthenticateClientResponseEs9.fromAsn1(node, tag: tag);
    }
    return AuthenticateClientResponseEs9();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateClientResponseEs9(');
    final authenticateClientOkVal = authenticateClientOk;
    if (authenticateClientOkVal != null) {
      final val = authenticateClientOkVal;
      if (val is Uint8List) {
        buffer.writeln('  authenticateClientOk: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  authenticateClientOk: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final authenticateClientErrorVal = authenticateClientError;
    if (authenticateClientErrorVal != null) {
      final val = authenticateClientErrorVal;
      if (val is Uint8List) {
        buffer.writeln('  authenticateClientError: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  authenticateClientError: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final authenticateClientOkRpmVal = authenticateClientOkRpm;
    if (authenticateClientOkRpmVal != null) {
      final val = authenticateClientOkRpmVal;
      if (val is Uint8List) {
        buffer.writeln('  authenticateClientOkRpm: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  authenticateClientOkRpm: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final authenticateClientOkDeviceChangeVal =
        authenticateClientOkDeviceChange;
    if (authenticateClientOkDeviceChangeVal != null) {
      final val = authenticateClientOkDeviceChangeVal;
      if (val is Uint8List) {
        buffer.writeln('  authenticateClientOkDeviceChange: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  authenticateClientOkDeviceChange: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final authenticateClientOkDelayedDeviceChangeVal =
        authenticateClientOkDelayedDeviceChange;
    if (authenticateClientOkDelayedDeviceChangeVal != null) {
      final val = authenticateClientOkDelayedDeviceChangeVal;
      if (val is Uint8List) {
        buffer.writeln(
          '  authenticateClientOkDelayedDeviceChange: ${_hex(val)},',
        );
      } else {
        buffer.writeln(
          '  authenticateClientOkDelayedDeviceChange: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateClientOk {
  final TransactionId? transactionId;
  final StoreMetadataRequest? profileMetadata;
  final SmdpSigned2? smdpSigned2;
  final Uint8List? smdpSignature2;
  final Certificate? smdpCertificate;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateClientOk._({
    this.transactionId,
    this.profileMetadata,
    this.smdpSigned2,
    this.smdpSignature2,
    this.smdpCertificate,
    this.asn1,
  });

  const AuthenticateClientOk({
    TransactionId? transactionId,
    StoreMetadataRequest? profileMetadata,
    SmdpSigned2? smdpSigned2,
    Uint8List? smdpSignature2,
    Certificate? smdpCertificate,
  }) : this._(
         transactionId: transactionId,
         profileMetadata: profileMetadata,
         smdpSigned2: smdpSigned2,
         smdpSignature2: smdpSignature2,
         smdpCertificate: smdpCertificate,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (profileMetadata != null)
      children.add((profileMetadata as dynamic).encode(tag: 0xBF25));
    if (smdpSigned2 != null)
      children.add((smdpSigned2 as dynamic).encode(tag: 0x30));
    if (smdpSignature2 != null)
      children.add(_asn1EncodeRaw(0x5F37, smdpSignature2!));
    if (smdpCertificate != null)
      children.add((smdpCertificate as dynamic).encode(tag: 0x30));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateClientOk.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    StoreMetadataRequest? profileMetadata;
    SmdpSigned2? smdpSigned2;
    Uint8List? smdpSignature2;
    Certificate? smdpCertificate;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xBF25);
    if (found >= 0) {
      final child = node!.children[found];
      profileMetadata = StoreMetadataRequest.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSigned2 = SmdpSigned2.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSignature2 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      smdpCertificate = Certificate.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    return AuthenticateClientOk._(
      transactionId: transactionId,
      profileMetadata: profileMetadata,
      smdpSigned2: smdpSigned2,
      smdpSignature2: smdpSignature2,
      smdpCertificate: smdpCertificate,
      asn1: node, // Store original node
    );
  }

  static AuthenticateClientOk decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return AuthenticateClientOk.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return AuthenticateClientOk.fromAsn1(node, tag: tag);
    }
    return AuthenticateClientOk();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateClientOk(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final profileMetadataVal = profileMetadata;
    if (profileMetadataVal != null) {
      final val = profileMetadataVal;
      if (val is Uint8List) {
        buffer.writeln('  profileMetadata: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  profileMetadata: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSigned2Val = smdpSigned2;
    if (smdpSigned2Val != null) {
      final val = smdpSigned2Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSigned2: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSigned2: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSignature2Val = smdpSignature2;
    if (smdpSignature2Val != null) {
      final val = smdpSignature2Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSignature2: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSignature2: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpCertificateVal = smdpCertificate;
    if (smdpCertificateVal != null) {
      final val = smdpCertificateVal;
      if (val is Uint8List) {
        buffer.writeln('  smdpCertificate: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpCertificate: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateClientOkRpm {
  final TransactionId? transactionId;
  final SmdpSigned3? smdpSigned3;
  final Uint8List? smdpSignature3;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateClientOkRpm._({
    this.transactionId,
    this.smdpSigned3,
    this.smdpSignature3,
    this.asn1,
  });

  const AuthenticateClientOkRpm({
    TransactionId? transactionId,
    SmdpSigned3? smdpSigned3,
    Uint8List? smdpSignature3,
  }) : this._(
         transactionId: transactionId,
         smdpSigned3: smdpSigned3,
         smdpSignature3: smdpSignature3,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (smdpSigned3 != null)
      children.add((smdpSigned3 as dynamic).encode(tag: 0x30));
    if (smdpSignature3 != null)
      children.add(_asn1EncodeRaw(0x5F37, smdpSignature3!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateClientOkRpm.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    SmdpSigned3? smdpSigned3;
    Uint8List? smdpSignature3;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSigned3 = SmdpSigned3.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSignature3 = child.value;
      childIdx = found + 1;
    }
    return AuthenticateClientOkRpm._(
      transactionId: transactionId,
      smdpSigned3: smdpSigned3,
      smdpSignature3: smdpSignature3,
      asn1: node, // Store original node
    );
  }

  static AuthenticateClientOkRpm decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return AuthenticateClientOkRpm.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return AuthenticateClientOkRpm.fromAsn1(node, tag: tag);
    }
    return AuthenticateClientOkRpm();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateClientOkRpm(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSigned3Val = smdpSigned3;
    if (smdpSigned3Val != null) {
      final val = smdpSigned3Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSigned3: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSigned3: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSignature3Val = smdpSignature3;
    if (smdpSignature3Val != null) {
      final val = smdpSignature3Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSignature3: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSignature3: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateClientOkDeviceChange {
  final TransactionId? transactionId;
  final SmdpSigned4? smdpSigned4;
  final Uint8List? smdpSignature4;
  final LocalisedTextMessage? serviceProviderMessageForDc;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateClientOkDeviceChange._({
    this.transactionId,
    this.smdpSigned4,
    this.smdpSignature4,
    this.serviceProviderMessageForDc,
    this.asn1,
  });

  const AuthenticateClientOkDeviceChange({
    TransactionId? transactionId,
    SmdpSigned4? smdpSigned4,
    Uint8List? smdpSignature4,
    LocalisedTextMessage? serviceProviderMessageForDc,
  }) : this._(
         transactionId: transactionId,
         smdpSigned4: smdpSigned4,
         smdpSignature4: smdpSignature4,
         serviceProviderMessageForDc: serviceProviderMessageForDc,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (smdpSigned4 != null)
      children.add((smdpSigned4 as dynamic).encode(tag: 0x30));
    if (smdpSignature4 != null)
      children.add(_asn1EncodeRaw(0x5F37, smdpSignature4!));
    if (serviceProviderMessageForDc != null)
      children.add((serviceProviderMessageForDc as dynamic).encode(tag: 0xA1));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateClientOkDeviceChange.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    SmdpSigned4? smdpSigned4;
    Uint8List? smdpSignature4;
    LocalisedTextMessage? serviceProviderMessageForDc;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSigned4 = SmdpSigned4.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSignature4 = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0xA1);
    if (found >= 0) {
      final child = node!.children[found];
      serviceProviderMessageForDc = LocalisedTextMessage.decode(
        child,
        tag: child.tag,
      );
      childIdx = found + 1;
    }
    return AuthenticateClientOkDeviceChange._(
      transactionId: transactionId,
      smdpSigned4: smdpSigned4,
      smdpSignature4: smdpSignature4,
      serviceProviderMessageForDc: serviceProviderMessageForDc,
      asn1: node, // Store original node
    );
  }

  static AuthenticateClientOkDeviceChange decode(dynamic data, {int? tag}) {
    if (data is Asn1Node)
      return AuthenticateClientOkDeviceChange.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return AuthenticateClientOkDeviceChange.fromAsn1(node, tag: tag);
    }
    return AuthenticateClientOkDeviceChange();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateClientOkDeviceChange(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSigned4Val = smdpSigned4;
    if (smdpSigned4Val != null) {
      final val = smdpSigned4Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSigned4: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSigned4: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSignature4Val = smdpSignature4;
    if (smdpSignature4Val != null) {
      final val = smdpSignature4Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSignature4: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSignature4: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final serviceProviderMessageForDcVal = serviceProviderMessageForDc;
    if (serviceProviderMessageForDcVal != null) {
      final val = serviceProviderMessageForDcVal;
      if (val is Uint8List) {
        buffer.writeln('  serviceProviderMessageForDc: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  serviceProviderMessageForDc: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class AuthenticateClientOkDelayedDeviceChange {
  final TransactionId? transactionId;
  final SmdpSigned6? smdpSigned6;
  final Uint8List? smdpSignature6;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const AuthenticateClientOkDelayedDeviceChange._({
    this.transactionId,
    this.smdpSigned6,
    this.smdpSignature6,
    this.asn1,
  });

  const AuthenticateClientOkDelayedDeviceChange({
    TransactionId? transactionId,
    SmdpSigned6? smdpSigned6,
    Uint8List? smdpSignature6,
  }) : this._(
         transactionId: transactionId,
         smdpSigned6: smdpSigned6,
         smdpSignature6: smdpSignature6,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (transactionId != null)
      children.add(_asn1EncodeRaw(0x80, transactionId!));
    if (smdpSigned6 != null)
      children.add((smdpSigned6 as dynamic).encode(tag: 0x30));
    if (smdpSignature6 != null)
      children.add(_asn1EncodeRaw(0x5F37, smdpSignature6!));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory AuthenticateClientOkDelayedDeviceChange.fromAsn1(
    Asn1Node node, {
    int? tag,
  }) {
    final defaultTag = 0x30;
    TransactionId? transactionId;
    SmdpSigned6? smdpSigned6;
    Uint8List? smdpSignature6;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x80);
    if (found >= 0) {
      final child = node!.children[found];
      transactionId = child.value;
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x30);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSigned6 = SmdpSigned6.decode(child, tag: child.tag);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x5F37);
    if (found >= 0) {
      final child = node!.children[found];
      smdpSignature6 = child.value;
      childIdx = found + 1;
    }
    return AuthenticateClientOkDelayedDeviceChange._(
      transactionId: transactionId,
      smdpSigned6: smdpSigned6,
      smdpSignature6: smdpSignature6,
      asn1: node, // Store original node
    );
  }

  static AuthenticateClientOkDelayedDeviceChange decode(
    dynamic data, {
    int? tag,
  }) {
    if (data is Asn1Node)
      return AuthenticateClientOkDelayedDeviceChange.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null)
        return AuthenticateClientOkDelayedDeviceChange.fromAsn1(node, tag: tag);
    }
    return AuthenticateClientOkDelayedDeviceChange();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('AuthenticateClientOkDelayedDeviceChange(');
    final transactionIdVal = transactionId;
    if (transactionIdVal != null) {
      final val = transactionIdVal;
      if (val is Uint8List) {
        buffer.writeln('  transactionId: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  transactionId: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSigned6Val = smdpSigned6;
    if (smdpSigned6Val != null) {
      final val = smdpSigned6Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSigned6: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSigned6: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final smdpSignature6Val = smdpSignature6;
    if (smdpSignature6Val != null) {
      final val = smdpSignature6Val;
      if (val is Uint8List) {
        buffer.writeln('  smdpSignature6: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  smdpSignature6: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}

class ExtendedCardResource {
  final int? installedApplication;
  final int? freeNonVolatileMemory;
  final int? freeVolatileMemory;
  final Asn1Node? asn1; // Original ASN.1 node for byte-perfect re-encoding

  const ExtendedCardResource._({
    this.installedApplication,
    this.freeNonVolatileMemory,
    this.freeVolatileMemory,
    this.asn1,
  });

  const ExtendedCardResource({
    int? installedApplication,
    int? freeNonVolatileMemory,
    int? freeVolatileMemory,
  }) : this._(
         installedApplication: installedApplication,
         freeNonVolatileMemory: freeNonVolatileMemory,
         freeVolatileMemory: freeVolatileMemory,
         asn1: null,
       );

  Uint8List encode({int? tag}) {
    // If we have the original ASN.1 node, re-encode it for byte-perfect output
    if (asn1 != null) {
      return asn1!.encodedBytes;
    }
    final children = <Uint8List>[];
    final defaultTag = 0x30;
    if (installedApplication != null)
      children.add(_asn1EncodeRaw(0x81, _asn1EncodeInt(installedApplication!)));
    if (freeNonVolatileMemory != null)
      children.add(
        _asn1EncodeRaw(0x82, _asn1EncodeInt(freeNonVolatileMemory!)),
      );
    if (freeVolatileMemory != null)
      children.add(_asn1EncodeRaw(0x83, _asn1EncodeInt(freeVolatileMemory!)));
    return _asn1EncodeConstructed(tag ?? defaultTag, children);
  }

  factory ExtendedCardResource.fromAsn1(Asn1Node node, {int? tag}) {
    final defaultTag = 0x30;
    int? installedApplication;
    int? freeNonVolatileMemory;
    int? freeVolatileMemory;
    int childIdx = 0;
    int found;
    found = _seekTag(node!.children, childIdx, 0x81);
    if (found >= 0) {
      final child = node!.children[found];
      installedApplication = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x82);
    if (found >= 0) {
      final child = node!.children[found];
      freeNonVolatileMemory = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    found = _seekTag(node!.children, childIdx, 0x83);
    if (found >= 0) {
      final child = node!.children[found];
      freeVolatileMemory = _asn1DecodeInt(child.value);
      childIdx = found + 1;
    }
    return ExtendedCardResource._(
      installedApplication: installedApplication,
      freeNonVolatileMemory: freeNonVolatileMemory,
      freeVolatileMemory: freeVolatileMemory,
      asn1: node, // Store original node
    );
  }

  static ExtendedCardResource decode(dynamic data, {int? tag}) {
    if (data is Asn1Node) return ExtendedCardResource.fromAsn1(data, tag: tag);
    if (data is Uint8List) {
      final node = SimpleAsn1Reader(data).readNext();
      if (node != null) return ExtendedCardResource.fromAsn1(node, tag: tag);
    }
    return ExtendedCardResource();
  }

  @override
  String toString() {
    final buffer = StringBuffer();
    buffer.writeln('ExtendedCardResource(');
    final installedApplicationVal = installedApplication;
    if (installedApplicationVal != null) {
      final val = installedApplicationVal;
      if (val is Uint8List) {
        buffer.writeln('  installedApplication: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  installedApplication: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final freeNonVolatileMemoryVal = freeNonVolatileMemory;
    if (freeNonVolatileMemoryVal != null) {
      final val = freeNonVolatileMemoryVal;
      if (val is Uint8List) {
        buffer.writeln('  freeNonVolatileMemory: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  freeNonVolatileMemory: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    final freeVolatileMemoryVal = freeVolatileMemory;
    if (freeVolatileMemoryVal != null) {
      final val = freeVolatileMemoryVal;
      if (val is Uint8List) {
        buffer.writeln('  freeVolatileMemory: ${_hex(val)},');
      } else {
        buffer.writeln(
          '  freeVolatileMemory: ${val.toString().replaceAll("\n", "\n  ")},',
        );
      }
    }
    buffer.write(')');
    return buffer.toString();
  }
}
