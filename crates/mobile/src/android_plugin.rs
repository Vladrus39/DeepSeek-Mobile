//! Android Gradle artifact metadata for the Dioxus mobile bundle.
//!
//! `dx build --android` generates a temporary Android project under `target/dx`.
//! This metadata asks the Dioxus CLI to copy the repository's Kotlin bridge module
//! into that generated project and add it as a Gradle dependency.

#[manganis::ffi("../../android/bridge")]
extern "Kotlin" {
    pub type DeepSeekMobileHostCoordinator;
}
