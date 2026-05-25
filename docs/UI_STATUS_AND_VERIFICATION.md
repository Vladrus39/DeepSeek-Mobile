# Mobile UI status and verification

**Updated:** 2026-05-25

## Current UI reality

The mobile UI is no longer a rough mock. It contains the main cockpit surfaces required for the coding agent:

- onboarding and settings, including Termux workspace activation;
- chat timeline and approval cards;
- Files, Snapshots, Diagnostics, PC Host, Terminal, Git, Tasks, MCP and Skills panels;
- native bridge status banners for Android callbacks;
- bottom navigation and a drawer for deeper navigation.

The latest UI pass tightened the global chrome so the app no longer shows misleading static status:

- top status chips show API and PC state;
- the active workspace/project is visible in the main header and drawer, including a saved Termux workspace when no PC workspace is active;
- drawer and bottom navigation badges are driven by live state: PC, approvals, diagnostics, dirty Git files, running tasks and native-callback waiting;
- bottom navigation is horizontally scrollable instead of squeezing nine actions into a fixed phone width;
- the old invalid `space_around` CSS value was removed.

## Visual design intent

The intended v1 visual direction is a dark, dense coding cockpit rather than a consumer chat clone:

- high-contrast dark surfaces for long coding sessions;
- blue for primary navigation and active state;
- green for healthy/connected states;
- red/yellow for errors, warnings and pending approvals;
- compact cards and status chips so the phone screen keeps tool state visible without opening multiple menus.

This is functional and coherent, but it is still an engineering UI. The final polish pass should happen on a real Android device or emulator after the Dioxus Android host is wired.

## Verification status

Verified now:

- Rust/Dioxus code compiles with `cargo +stable-x86_64-pc-windows-msvc check -p deepseek-mobile --all-targets`.
- Existing component/state tests cover drawer sections, navigation items, panel states and the new dynamic chrome badges.
- Static source audit found no active UI placeholder implementation beyond normal input placeholder text.

Not yet fully verified:

- Real Android rendering through `dx serve --platform android` or an APK install.
- Touch ergonomics on a physical phone.
- Native callback flows end-to-end from Kotlin/Dioxus host into Rust state.

Current local blocker for visual device verification:

- `dx` / Dioxus CLI is not installed in the current Windows environment.
- Android SDK and JDK are present, but Android Rust targets and Dioxus CLI setup still need to be completed before a real device/emulator UI pass.

## Required final UI verification before v1

1. Install matching Dioxus CLI for the project Dioxus version.
2. Add Android Rust targets required by the Dioxus Android build.
3. Run `dx serve --platform android` against an emulator or device.
4. Verify these flows manually:
   - onboarding, settings save and Termux workspace activation;
   - chat send and streaming timeline;
   - approval card review;
   - Files panel local and active-PC browsing;
   - PC pairing export/import/discovery status;
   - Termux command callback continuation;
   - terminal open/output/close events;
   - bottom nav and drawer readability on small and large phone widths.

## Conclusion

The UI is implemented enough to be called a real cockpit, not a placeholder. It is not yet honest to call it visually finished until the Android host/device verification pass is completed.
