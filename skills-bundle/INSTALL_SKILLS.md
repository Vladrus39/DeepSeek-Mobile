## DeepSeek-Mobile skills bundle install (Android)

This repo ships a ready-made skills bundle under `skills-bundle/`. The app discovers skills by scanning **the `skills/` directory** under your configured data dir:

- **Internal app files (preferred)**: `run-as com.deepseek.mobile` → `files/deepseek-mobile/skills/`
- **External app files (easy to push)**: `/sdcard/Android/data/com.deepseek.mobile/files/deepseek-mobile/skills/`

The registry loads any first-level subdirectory that contains a `SKILL.md` file:

- `.../skills/<skill-folder>/SKILL.md`

### Requirements

- **USB debugging enabled**
- **ADB available** (`tools/android/sdk/platform-tools/adb.exe` in this repo, or `adb` in PATH)
- For copying into internal files: the app must be **debuggable** so `adb shell run-as com.deepseek.mobile ...` works.

### Option A (recommended): use the helper script

From Windows PowerShell in the repo root:

```powershell
. .\tools\android\env.ps1
.\scripts\push-skills-to-device.ps1
```

It will:

- create `/sdcard/Android/data/com.deepseek.mobile/files/deepseek-mobile/skills/`
- push the bundle skills there
- attempt to copy into internal `run-as` dir (`files/deepseek-mobile/skills/`) when possible

### Option B: manual install with adb (external files dir)

From repo root:

```powershell
$pkg = "com.deepseek.mobile"
$dst = "/sdcard/Android/data/$pkg/files/deepseek-mobile/skills"

adb shell "mkdir -p $dst"
adb push ".\skills-bundle\skills\." $dst
adb shell "ls -la $dst"
```

Then open the app → Skills tab → refresh/reopen.

### Option C: manual install into internal files (run-as)

This keeps skills in the app sandbox (best for privacy and predictable discovery).

1) Push to a temp location first:

```powershell
$tmp = "/data/local/tmp/deepseek-mobile-skills"
adb shell "rm -rf $tmp && mkdir -p $tmp"
adb push ".\skills-bundle\skills\." $tmp
adb shell "find $tmp -maxdepth 2 -type f -name SKILL.md -print"
```

2) Copy into the app’s internal files via `run-as`:

```powershell
$pkg = "com.deepseek.mobile"
adb shell "run-as $pkg mkdir -p files/deepseek-mobile/skills"
adb shell "run-as $pkg sh -lc 'rm -rf files/deepseek-mobile/skills/*'"
adb shell "run-as $pkg sh -lc 'cp -R /data/local/tmp/deepseek-mobile-skills/* files/deepseek-mobile/skills/'"
adb shell "run-as $pkg sh -lc 'find files/deepseek-mobile/skills -maxdepth 2 -name SKILL.md -print'"
```

If `run-as` fails with "not debuggable", use Option A or B and keep skills in external files.

### Verify discovery from the device

You can quickly verify the internal skills directory exists and contains the expected `SKILL.md`:

```powershell
$pkg = "com.deepseek.mobile"
adb shell "run-as $pkg sh -lc 'ls -la files/deepseek-mobile/skills && find files/deepseek-mobile/skills -maxdepth 2 -name SKILL.md -print'"
```

Or check external files:

```powershell
$pkg = "com.deepseek.mobile"
adb shell "find /sdcard/Android/data/$pkg/files/deepseek-mobile/skills -maxdepth 2 -name SKILL.md -print"
```

