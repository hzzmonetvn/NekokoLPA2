plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// Community builds always use the committed public key. Local properties and
// NLPA_* signing overrides belong to private brand builds, not this recipe.

android {
    namespace = "ee.nekoko.nlpa2"
    compileSdk = 36

    buildFeatures {
        buildConfig = true
        // The QRTR user service is described by an AIDL interface.
        aidl = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
        isCoreLibraryDesugaringEnabled = true
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_11.toString()
    }

    // AGP encrypts this optional record with a fresh nonce. It is outside the
    // signed payload and makes otherwise identical APKs differ between builds.
    // Public Community APKs must be reproducible without Google Play metadata.
    dependenciesInfo {
        includeInApk = false
        includeInBundle = false
    }

    packagingOptions {
        jniLibs {
            useLegacyPackaging = true
        }
    }

    signingConfigs {
        create("community") {
            storeFile = file("community.jks")
            storePassword = "CommunityKey"
            keyAlias = "communitykey"
            keyPassword = "CommunityKey"
        }
    }

    flavorDimensions += "version"

    val buildOnlyAbi = providers.gradleProperty("buildAbi").orNull?.takeUnless { it.isEmpty() || it == "all" }
    splits {
        abi {
            isEnable = true
            reset()
            if (buildOnlyAbi != null) {
                include(buildOnlyAbi)
            } else {
                include("armeabi-v7a", "arm64-v8a")
            }
            isUniversalApk = buildOnlyAbi == null
        }
    }

    productFlavors {
        create("community") {
            dimension = "version"
            applicationId = "ee.nekoko.nlpa2.open"
            manifestPlaceholders["appName"] = "NekokoLPA 2 Community"
            buildConfigField("boolean", "PRIVILEGED_BUILD", "false")
            signingConfig = signingConfigs.getByName("community")
        }

        create("privileged") {
            dimension = "version"
            applicationId = "ee.nekoko.nlpa2"
            manifestPlaceholders["appName"] = "NekokoLPA 2 Community Rooted"
            buildConfigField("boolean", "PRIVILEGED_BUILD", "true")
            signingConfig = signingConfigs.getByName("community")
        }
    }

    // Bundle TS.43 entitlement DB and profile size reference data into Android assets,
    // and share the Community dolphin artwork with public rooted builds.
    sourceSets {
        getByName("main") {
            assets.srcDirs("src/main/assets", "../../assets", "../../data")
        }
        getByName("privileged") {
            res.setSrcDirs(listOf("src/community/res"))
        }
    }

    defaultConfig {
        applicationId = "ee.nekoko.nlpa2.open"
        minSdk = 28
        targetSdk = 36
        versionCode = 606
        versionName = "2.5.0"
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )

            // Force version code to match defaultConfig.versionCode, preventing ABI splitting logic from changing it
            applicationVariants.all {
                outputs.all {
                    (this as? com.android.build.gradle.internal.api.ApkVariantOutputImpl)?.versionCodeOverride = 606
                }
            }
        }
        debug {
            signingConfig = signingConfigs.getByName("community")
        }
    }
}

dependencies {
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.4")

    // Only the `shell` user may open a QRTR socket, so the app asks Shizuku to
    // run a service there and takes the descriptor back over Binder.
    implementation("dev.rikka.shizuku:api:13.1.5")
    implementation("dev.rikka.shizuku:provider:13.1.5")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20240303")
}
