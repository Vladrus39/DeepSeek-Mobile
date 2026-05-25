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
include(":bridge")
project(":bridge").projectDir = file("bridge")
include(":app")
project(":app").projectDir = file("app")
