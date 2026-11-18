package ee.nekoko.nlpa2

import android.content.Context
import android.content.pm.PackageManager
import android.content.pm.Signature
import android.os.Build
import java.security.MessageDigest

internal class CertificatePlugin {
    private var context: Context? = null

    fun attach(appContext: Context) {
        context = appContext.applicationContext
    }

    fun detach() {
        context = null
    }

    fun getInfo(): HostCallResult {
        val appContext =
            context ?: return HostCallResult.Error("NOT_ATTACHED", "Context unavailable")
        return try {
            val currentHashes = getHashes(getCurrentSignatures(appContext))
            val aramHashes = getHashes(getAramSignatures(appContext))
            HostCallResult.Success(
                mapOf(
                    "sha256" to currentHashes.sha256,
                    "sha1" to currentHashes.sha1,
                    "aramSha256" to aramHashes.sha256,
                    "aramSha1" to aramHashes.sha1,
                    "abis" to Build.SUPPORTED_ABIS.toList(),
                ),
            )
        } catch (e: Exception) {
            HostCallResult.Error("ERROR", e.message ?: "Failed to read certificate info")
        }
    }

    @Suppress("DEPRECATION")
    private fun getCurrentSignatures(appContext: Context): Array<Signature>? {
        val info =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                appContext.packageManager.getPackageInfo(
                    appContext.packageName,
                    PackageManager.GET_SIGNING_CERTIFICATES,
                )
            } else {
                appContext.packageManager.getPackageInfo(
                    appContext.packageName,
                    PackageManager.GET_SIGNATURES,
                )
            }

        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            info.signingInfo?.apkContentsSigners
        } else {
            info.signatures
        }
    }

    @Suppress("DEPRECATION")
    private fun getAramSignatures(appContext: Context): Array<Signature>? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.P) {
            return appContext.packageManager
                .getPackageInfo(appContext.packageName, PackageManager.GET_SIGNATURES)
                .signatures
        }

        val signingInfo =
            appContext.packageManager
                .getPackageInfo(
                    appContext.packageName,
                    PackageManager.GET_SIGNING_CERTIFICATES,
                )
                .signingInfo ?: return null

        // Existing Android SecureElement releases authorize the oldest signer after a key
        // rotation, while newer native OMAPI releases authorize the current signer. Returning
        // the verified signing lineage lets the ARA-M page cover both implementations.
        return if (signingInfo.hasMultipleSigners()) {
            signingInfo.apkContentsSigners
        } else {
            signingInfo.signingCertificateHistory?.takeIf { it.isNotEmpty() }
                ?: signingInfo.apkContentsSigners
        }
    }

    private fun getHashes(signatures: Array<Signature>?): CertificateHashes {
        val certificates = signatures.orEmpty().map { it.toByteArray() }
        return CertificateHashes(
            sha256 = certificates.map { getHash(it, "SHA-256") }.distinct(),
            sha1 = certificates.map { getHash(it, "SHA-1") }.distinct(),
        )
    }

    private fun getHash(cert: ByteArray, algorithm: String): String {
        val md = MessageDigest.getInstance(algorithm)
        val digest = md.digest(cert)
        return digest.joinToString("") { "%02X".format(it) }
    }

    private data class CertificateHashes(val sha256: List<String>, val sha1: List<String>)
}
