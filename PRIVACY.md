# Privacy Statement

This statement describes, in technical and binding terms, what NB SelfCare does with your data. It is enforced by the
code and by automated checks that run on every change; where a guarantee depends on a check, the check is named. The
detailed Spanish reference is [docs/audit/privacy.md](docs/audit/privacy.md).

## 1. Data sovereignty

* **Everything is local.** Your profile, meals, measurements, treatments, journal, fasts and settings live in your
  user directories (`~/.local/share/nb-selfcare` on Linux, `%APPDATA%\NB SelfCare` on Windows,
  `~/Library/Application Support/NB SelfCare` on macOS). There is no server operated by the project that receives them.
* **No account.** No feature requires registration, an email address, a licence key or a sign-in.
* **Portability.** A backup file holds every record and restores it on another computer or on NB SelfCare Mobile.
  Backups contain health data without encryption; store them in a protected place.

## 2. Zero telemetry

NB SelfCare contains **no** analytics, usage tracking, crash reporting, advertising identifiers or remote logging. The
module named `telemetry` writes local log files only, and it redacts content: logs never contain names, paths,
journal text, treatments or keys. This is enforced by `scripts/check-registro.mjs` on every change.

## 3. Network access

Every outgoing connection passes through one module, `src-tauri/src/net.rs`. Any network client used elsewhere fails
the build checks (`scripts/check-network.mjs`). The gate enforces, in order:

1. **The global switch.** When it is off, no connection is opened at all.
2. **Per-service consent.** Each service has its own switch in Settings and states what it receives.
3. **Allowed hosts.** Each service can only reach its own hosts.

| Service | What is sent | When |
|---|---|---|
| Open Food Facts | The barcode you look up, never your records | When you look up a code that is not in the local catalogue |
| Updates | Installed version and platform | When you check for updates |
| Phone link | Your records, only to your paired phones on the local network | While the link is allowed and a paired phone syncs |

## 4. Local network

* **NB SelfCare Mobile:** the desktop listens only on a private local address, with TLS and a pinned certificate.
  Pairing needs a one-time code valid for five minutes. Each phone keeps a revocable credential; removing a phone
  closes its access at once. Health records travel only between your paired devices.

## 5. Secrets

The link identity and the paired phones are stored in the operating-system keyring, never in settings files, the
database, backups or logs.

## 6. Notifications

Operating-system notifications can be visible to others. Discreet reminders avoid treatment names. The application
does not guarantee delivery of a medical reminder.

## 7. Institutional use

This application does not report health or usage data to any institution. Institutional services, if any are
deployed, are separate products under their own agreements and cannot read local data through this application.

## 8. Changes

Any change to this statement is made in the same change as the code it describes, and is visible in the repository
history.
