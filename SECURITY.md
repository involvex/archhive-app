# Security Policy

## Supported versions

Security fixes are applied to the latest release on `main`. Older tagged releases may not receive backports unless noted in release notes.

| Version               | Supported   |
| --------------------- | ----------- |
| Latest release (`v*`) | Yes         |
| `main` (pre-release)  | Best effort |
| Older tags            | No          |

## Reporting a vulnerability

**Please do not open public GitHub issues for security vulnerabilities.**

Use one of these channels:

1. **GitHub Security Advisories (preferred)**  
   [Report a vulnerability](https://github.com/involvex/archhive-app/security/advisories/new) on this repository if private reporting is enabled.

2. **Maintainer contact**  
   If private advisories are unavailable, open a minimal public issue asking for a secure contact channel only (no exploit details in the issue body).

Include as much of the following as you can:

- Description of the issue and impact
- Steps to reproduce or a proof of concept
- Affected version(s) and platform (desktop Windows, Android, LAN client, etc.)
- Any suggested mitigation

We aim to acknowledge reports within **7 days** and provide a status update within **30 days**. Critical issues may be patched sooner.

## Scope

### In scope

- Remote code execution or arbitrary command execution in the desktop app
- Authentication or authorization bypass on the LAN REST API (`/api/*`)
- Cookie vault encryption failures or plaintext credential leakage at rest
- Path traversal or unsafe file writes outside the configured library / data directories
- Tauri capability / permission misconfigurations that expose privileged APIs to untrusted content
- Supply-chain issues in release artifacts published from this repository

### Out of scope

- Vulnerabilities in third-party sites ArcHive browses or downloads from
- Issues that require physical access to an unlocked device with ArcHive already running
- LAN attacks when the user has intentionally enabled LAN with a weak or shared token on an untrusted network (mitigate with a strong token and trusted LAN)
- Social engineering or phishing against users
- Denial of service against external sites via download queue abuse
- Findings in dependencies with no practical exploit path in ArcHive (still welcome; we may forward upstream)

## Safe harbor

We support good-faith security research. Do not access data that is not yours, disrupt services for other users, or violate applicable laws. We will not pursue legal action for research that follows this policy and gives us reasonable time to fix issues before public disclosure.

## Security practices in ArcHive

Understanding these areas helps write useful reports:

| Area             | Notes                                                                                                                                                                                                                        |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Cookie vault** | Site cookies are encrypted at rest (AES-256-GCM). Plaintext cookie files may exist briefly for yt-dlp `--cookies` usage.                                                                                                     |
| **LAN server**   | Bearer token required for all endpoints except `/api/health`. Default token is randomly generated; treat it like a password. `/api/files/stream` and `/api/scenes/{id}/media` expose library files to anyone with the token. |
| **Downloads**    | External URLs are passed to yt-dlp, gallery-dl, or direct HTTP fetchers. Command invocation uses argument arrays, not shell interpolation.                                                                                   |
| **CSP**          | Content Security Policy is currently disabled in `tauri.conf.json` for development flexibility. Hardening before wide distribution is planned.                                                                               |
| **Plugins**      | TypeScript plugins in `plugins/` run in the app context. Only install plugins from sources you trust.                                                                                                                        |

## Coordinated disclosure

We prefer coordinated disclosure. After a fix is released, we are happy to credit reporters who want attribution (unless you prefer to remain anonymous).

Thank you for helping keep ArcHive and its users safe.
