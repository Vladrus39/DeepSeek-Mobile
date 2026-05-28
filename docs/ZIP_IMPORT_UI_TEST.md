# ZIP import — manual UI test (system picker)

**Device tested:** Samsung `RFCNC0PWD4E` · `com.deepseek.mobile`  
**Headless import:** PASS via `scripts/device-e2e-zip-import.ps1` (no picker).

## Prerequisites

- App installed from latest `dx build android` (includes `onWindowFocusChanged` workaround in `android/MainActivity.kt`).
- A small `.zip` on the phone (e.g. push `target/import-test.zip` to Downloads).

## Steps (~1 min)

1. Open DeepSeek Mobile → **Files** (bottom nav).
2. Under **Project import/export**, tap **Import ZIP**.
3. In the system file picker, choose your `.zip` (Samsung Files → Downloads is fine).
4. Return to the app. Expect status: **Project archive imported: …** and Files list refreshes.
5. Optional: export with **Export ZIP** → share sheet opens.

## If import fails

```powershell
adb logcat -c
# reproduce Import ZIP once
adb logcat -d | Select-String -Pattern "DocumentPicker|project import|AndroidRuntime|deepseek"
```

Common causes:

| Symptom | Likely cause |
|---------|----------------|
| Picker shows no `.zip` | Archive MIME was `application/octet-stream` — fixed in `DocumentPickerRequest::project_import()` (also accepts `application/x-zip-compressed`). Rebuild APK. |
| App frozen after picking | Old builds without `onWindowFocusChanged` override — rebuild. |
| «selected archive was not copied» | Picker cancelled or provider denied read — retry; pick from Downloads not cloud-only stub. |
| «project import failed» unzip error | Not a valid zip or path traversal in archive — use a normal project zip. |

## ADB helper (prepare test zip)

```powershell
.\scripts\device-e2e-zip-import.ps1 -Device RFCNC0PWD4E
adb push target\import-test.zip /sdcard/Download/import-test.zip
```

Then run the UI steps above and select `import-test.zip` in Downloads.
