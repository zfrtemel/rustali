# Security Policy

Rustali was started in response to real-world security failures in other web
ecosystems. We take reports seriously and respond quickly.

## Supported versions

| Version | Supported |
|---------|-----------|
| latest minor (0.x) | ✅ |
| older | ❌ — upgrade |

Until 1.0, only the latest published minor receives security fixes.

## Reporting a vulnerability

**Do not open a public issue.** Use GitHub's private vulnerability reporting on the
repository ("Security" tab → "Report a vulnerability") or e-mail
**security@rustali.dev** (placeholder until the domain is set).

Please include: affected crate and version, a minimal reproduction, impact
assessment, and whether you want credit in the advisory.

We aim to acknowledge within **48 hours**, provide an initial assessment within
**7 days**, and ship a fix with a RustSec advisory as soon as it is ready.
Coordinated disclosure embargo is 90 days by default.

## Security baseline (what the framework guarantees by default)

- All SQL is parameterised through `sea-query`; string-built SQL is a bug.
- Passwords are hashed with argon2id (OWASP parameters); plain comparison is impossible
  through the public API.
- Session cookies are `HttpOnly`, `Secure` (outside dev), `SameSite=Lax`, signed and
  encrypted with `APP_KEY`.
- CSRF protection is on for all state-changing SSR routes.
- Rate limiting ships enabled on authentication endpoints in the starter.
- `cargo deny` runs in CI against the RustSec advisory database.
