# Traverse Android Demo

This is the first checked-in Android demo surface for Traverse.

What it does:
- renders one approved expedition flow
- shows ordered runtime state updates
- shows the final trace summary and output panel
- assumes the runtime exists outside the app process

Current implementation note:
- the repo now contains the Android app structure, Compose source, and a deterministic fixture-driven rendering path
- local smoke validation is available through `bash scripts/ci/android_demo_smoke.sh`
- this machine does not currently have Gradle or an Android SDK configured, so native Android assembly is not part of the local green path here

Expected local build path on a machine with Android tooling:
- `cd apps/android-demo`
- `./gradlew :app:assembleDebug`

Fixture source:
- `app/src/main/assets/expedition-runtime-session.json`
