# Mobile UI status and verification

**Updated:** 2026-05-26

## Current UI reality

The mobile UI is a real coding cockpit, not a placeholder. It contains:

- onboarding and settings, including Termux workspace activation;
- chat timeline and approval cards;
- Files, Snapshots, Diagnostics, PC Host, Terminal, Git, Tasks, MCP and Skills panels;
- Files Import ZIP / Export ZIP controls;
- local durable tasks plus PC running-task sync/stop controls;
- native bridge status banners for Android callbacks;
- bottom navigation and drawer navigation;
- live API/PC/workspace chips and dynamic badges.

## Verified visually on Android

A physical device smoke test now passes:

| Item | Result |
|---|---|
| APK install | Pass |
| Launch | Pass |
| Android UI render | Pass |
| Crash after launch | None observed |
| Icon resources | Present in merged manifest |

Observed current screen on the test device:

- one-screen setup in Russian;
- API check ready;
- Agent mode check ready;
- Termux path check still pending until the path is saved/configured;
- RU/EN language toggle visible;
- no startup panic/crash.

Previously, with completed setup/saved workspace state, the main cockpit rendered with `DeepSeek Mobile`, `API OK`, `PC SETUP`, local workspace card, chat empty state and quick actions.

Local screenshot evidence:

```text
target/android-device-screenshot-publish.png
```

This screenshot is a local artifact and is not committed.

## Verified by code/tests

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` passes.
- `cargo +stable-x86_64-pc-windows-msvc test --workspace` passes.
- Existing component/state tests cover drawer sections, navigation items, panel states and dynamic chrome badges.
- Static source audit found no active UI placeholder implementation beyond normal input placeholder text.

## Visual design intent

The v1 direction remains a dark, dense coding cockpit rather than a consumer chat clone:

- high-contrast dark surfaces for long coding sessions;
- blue for primary navigation and active state;
- green for healthy/connected states;
- red/yellow for errors, warnings and pending approvals;
- compact cards and status chips so phone screen space remains useful.

## What still needs UI verification

The startup/setup screen is verified, but full UI completion requires deeper manual flow testing:

1. setup completion through API + Termux path and sandbox-only fallback;
2. settings save and Termux workspace activation;
3. chat send and streaming timeline;
4. approval card review;
5. Files local and active-PC browsing;
6. Files Import ZIP and Export ZIP/native share;
7. Tasks local records, PC running-task sync and PC stop action;
8. PC pairing export/import/discovery status;
9. Termux command callback continuation;
10. terminal open/output/close events;
11. bottom nav and drawer readability on small/large phone widths.

## Conclusion

The UI is implemented and the Android startup visual smoke test passes. It is not yet fair to call the whole UI polished until the deeper native flows and touch ergonomics are walked through on hardware.
