pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "DeepSeekMobile"

// :bridge is the canonical reusable Kotlin + resources module (JNI NativeBridge with dual "main"/"deepseek_mobile" load,
// TermuxResultReceiver, DeepSeek*Bridge coordinators for picker/discovery/Termux, adaptive launcher icons, FileProvider).
// It is injected into Dioxus-generated Android host builds via manganis::ffi in crates/mobile/src/android_plugin.rs
// and also depended on by the standalone :app for debug/testing.
include(":bridge")
project(":bridge").projectDir = file("bridge")

// :app is a standalone Gradle application module (ComponentActivity based, no Dioxus/Wry).
// Use it for direct `./gradlew :app:assembleDebug` testing of the native bridge + host coordinator
// *without* the full Dioxus mobile runtime. Production/release APKs are built with `dx build --android`
// (which uses the root android/MainActivity.kt + AndroidManifest.xml overlay + the :bridge library).
// The app module depends on :bridge for shared code and merged resources (icons, receiver, provider).
include(":app")
project(":app").projectDir = file("app")
