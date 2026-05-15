# DeepSeek PC Host

`deepseek-pc-host` is the background runtime process that runs on a trusted PC/laptop while the Android app acts as the control cockpit.

The host exposes the typed PC gateway protocol from `deepseek-mobile-core` over a local HTTP endpoint:

```text
POST /v1/gateway/request
```

Default bind address:

```text
127.0.0.1:8787
```

Useful environment variables:

```text
DEEPSEEK_PC_HOST_BIND=127.0.0.1:8787
DEEPSEEK_PC_HOST_ID=pc-local
DEEPSEEK_PC_HOST_LABEL=Developer PC
DEEPSEEK_PC_HOST_WORKSPACE=/absolute/path/to/project
DEEPSEEK_PC_HOST_WORKSPACE_ID=local
DEEPSEEK_PC_HOST_TOKEN=optional-shared-token
```

If `DEEPSEEK_PC_HOST_TOKEN` is set, requests must include:

```text
Authorization: Bearer <token>
```

Current first-pass capabilities:

- health
- list workspaces
- list directories inside granted workspace
- read files inside granted workspace
- write files inside granted workspace
- execute allowed developer commands inside granted workspace
- git status
- git diff

This is intentionally workspace-scoped. It must not expose the whole computer by default.
