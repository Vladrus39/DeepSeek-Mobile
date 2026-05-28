---
name: skill-authoring-discipline
description: Author SKILL.md bundles with frontmatter name/description; install under files/deepseek-mobile/skills/.
---

## Structure

```
skills/<folder-name>/SKILL.md
```

Frontmatter (required):

```yaml
---
name: my-skill-name
description: One line for the Skills UI and discovery.
---
```

Body: actionable checklists, commands, anti-patterns.

## Install on device

```powershell
.\scripts\push-skills-to-device.ps1 -Device <serial>
```

## Validation

- `name` must be unique across bundle.
- Folder name can differ from `name` but keep them aligned when possible.
- After push: open app → **Skills** → toggle ON → restart chat turn.

## Bundled location in repo

`skills-bundle/skills/` — ship with releases; push via adb.
