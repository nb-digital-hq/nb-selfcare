# AGENTS.md: operating contract for automated contributors

This file is the working contract of the repository for any coding agent (Claude, Gemini, Codex, Cursor, Copilot or
others) and for humans who want the same rules in one place. It applies **without anyone having to repeat it**.
Tool-specific configuration is never versioned in this repository and must never duplicate or contradict this file.

The project documentation is written in Spanish; this file, the root community files and the issue templates are in
English. Code identifiers and comments are in Spanish.

---

## 1. The rule that governs all others

**`docs/` is the single source of truth.** If the code and `docs/` disagree, there is a defect: either the code is
fixed or the document is changed through the decision process. A contradiction is never left alive.

A change with architectural or user-facing impact **is not finished** until the affected document reflects it, with
its status label up to date, in the same change.

## 2. Before writing code, in this order

| Step | Document | What to look for |
|---|---|---|
| 1 | [Constitution](docs/architecture/constitution.md) | The rules. Nothing may break them |
| 2 | [ADR index](docs/adr/README.md) | Why things are the way they are |
| 3 | [Architecture](docs/architecture/README.md) | Contracts of the module you are about to touch |
| 4 | [Product philosophy](docs/design/product-philosophy.md) | Mandatory when the change touches product or interface |

If the work depends on a decision that has not been taken, **do not start**: raise the decision first.

## 3. Repository architecture

```text
docs/              Single source of truth (Spanish). Start at docs/README.md
  adr/             Architecture Decision Records (000-template.md, 001-...)
  architecture/    Constitution, system overview, storage, IPC, mobile contract, sync contract and domain map,
                   NB Launcher, network, i18n, budgets, dependency policy
  audit/           Privacy, threat model, supply chain, performance
  design/          Product philosophy, UI conventions, branding
  developer/       Setup, code structure, standards, testing, CI, packaging, releases
  institution/     Relationship with the institutional offering
  legal/           Licensing, notices, trademark, data protection, security policy
  user/            End-user guide
  assets/          Branding, screenshots and video
src/               Interface: SvelteKit 2 + Svelte 5, static adapter
  lib/core/ipc/    The only access point to the runtime
  lib/bindings/    Types generated from Rust with ts-rs. Never edited by hand
  lib/i18n/        Spanish, English and Portuguese dictionaries
  lib/ui/          Components and views
src-tauri/         Runtime: Rust + Tauri 2
  src/db/          Persistence; migrations/ holds the embedded SQL (schema shared with SelfCare Mobile)
  src/comida.rs, cuerpo.rs, cuidado.rs, ayuno.rs, nibi.rs, perfil.rs, evolucion.rs, recetas.rs
                   Domain modules
  src/sincronizacion/ Phone link: envelope, fingerprints, pairing, TLS identity, domain adapters and service
  sincronizacion/  Sync domain specification; scripts/generar-sincronizacion.mjs derives schema 9 from it
  src/net.rs       **The only network exit.** Nothing leaves through any other path
  src/telemetry.rs Local logging with mandatory redaction. No remote telemetry exists
  tests/           Integration tests and the formats shared with SelfCare Mobile
scripts/           Guardians, packaging, notices, provenance and real-app QA
```

## 4. Commands

| Command | Purpose |
|---|---|
| `pnpm install` | Dependencies and Git hooks (`.githooks`) |
| `pnpm tauri dev` | Run the desktop application |
| `pnpm verify` | Lint, types, interface tests and guardians |
| `pnpm guardianes` | Only the guardians |
| `pnpm build` | Build the interface the runtime embeds (required before `cargo`) |
| `node scripts/generar-sincronizacion.mjs --movil ../NB_SelfCare_Mobile` | Regenerate schema 9 and the sync adapters for both applications |
| `scripts/empaquetar-linux.sh` | Linux packages inside Ubuntu 22.04 |

## 5. Non-negotiable rules

| Rule | Mechanism |
|---|---|
| Nothing to install outside the application; no executable looked up in `PATH` | `check-plataforma` |
| No Python interpreter, audio engine or model at runtime | `check-sin-python`, [ADR-002](docs/adr/002-no-python.md) |
| Eight destinations: Today, Food, Body, Care, Fasting, Nibi, Profile and Settings | [ADR-007](docs/adr/007-information-architecture.md) |
| No error travels as a string | `check-error-contract` |
| Bridge types are generated | `ts-rs` + `check-bindings` |
| No component calls the bridge outside `#lib/core/ipc` | Biome `noRestrictedImports`, `check-puente` |
| No bridge command is synchronous or runs on the window thread | `check-comandos` |
| Every collection crossing the bridge is bounded | `check-listas` |
| Every external path is canonicalised and bounded | `check-rutas` |
| Every write transaction opens `BEGIN IMMEDIATE`; migrations and restoration keep a prior backup | `check-transacciones`, storage tests |
| Nothing leaves the device outside `net.rs` or without consent | `check-network` |
| No log contains names, paths, journal text, treatments or keys | `check-registro` |
| No visible text outside the dictionaries; every key in the three languages | `check-i18n`, `Forma<Textos>` in `pnpm check` |
| Schema 9 and the sync adapters are generated from one specification, identical in both applications | `generar-sincronizacion --comprobar` |
| Medication and doses never earn coins; health estimates are not diagnoses | Domain tests |
| No literal colours, blur on permanent chrome or animation at rest | `check-ui` |
| The startup bundle stays within its budget | `check-paquete` |
| No new dependency without justification and an admitted licence | [dependency policy](docs/architecture/dependency-policy.md), `cargo deny` |

## 6. Privacy and zero telemetry

* **No telemetry, analytics, crash reporting or usage tracking** of any kind, including "anonymous" or opt-out
  variants.
* **Every outgoing connection goes through `src-tauri/src/net.rs`**, which checks the global switch, the per-service
  consent and the allowed hosts. The phone link listens only on a private local address with TLS.
* **The link identity and paired phones go to the system keyring**, never to settings, logs, backups or the database.
* **Personal records never reach logs or webview storage.**
* **No account or sign-in** is required by any feature.
* Real-app QA uses `NB_SELFCARE_TEST_DIR` with synthetic data only.

## 7. Verification

Run before declaring anything finished, without being asked:

```bash
pnpm verify
pnpm build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings   # warnings are errors
cargo test
```

Definition of done:

1. The acceptance criterion is met and another person can check it.
2. Tests for the added logic, and one regression test per fixed defect, named after the property they verify.
3. Verification is green.
4. If the change crosses the bridge, the generated bindings are current.
5. If it touches the sync domain, both applications and their shared fixtures change together.
6. If it has architectural or user-facing impact, `docs/` is updated in the same change.
7. If it adds visible text, it exists in Spanish, English and Portuguese.

## 8. Keeping `docs/` in sync

* Each fact has one owning document; others link to it.
* Every document declares its status on its first lines (**Implementado**, **Aceptado**, **Aplicable**).
* The structure of `docs/` is fixed: `adr`, `architecture`, `audit`, `design`, `developer`, `institution`, `legal`,
  `user`, `assets`.
* `node scripts/check-docs.mjs` rejects broken links, missing titles, missing status and broken tables.
* Documentation describes the product as it is. Development history, personal names and internal anecdotes do not
  belong in it; the reasoning behind a decision does, in impersonal form.

## 9. Language and style

* Spanish for code identifiers, comments, documentation in `docs/` and commit messages; English for the root
  community files and this file. Professional register, **no emoji** anywhere.
* Comments explain **why**, not what. When a decision is not obvious, the comment says what was discarded and why.

## 10. Commits

```bash
git commit -s -m "feat(comida): recetas por raciones"
```

* Conventional Commits; the `commit-msg` hook rejects anything else.
* `-s` is mandatory: Developer Certificate of Origin.
* The body explains **why** and what was verified, not the list of files.

## 11. What to do without asking

* Run verification; fix formatting, lints and failing tests.
* Write tests for added logic and a regression test for each fixed defect.
* Update the affected document and its status label.

## 12. What requires asking

* Approving or changing an ADR, or contradicting the Constitution.
* Adding a dependency or changing a licence.
* Any change to NB SelfCare Mobile or to the sync protocol.
* Publishing, pushing to a remote, or any outward-facing action.
* Any change that sends data off the device.

## 13. One agent on the working tree

**Two agentic tools must never write to this repository at the same time.** If modifications appear that you do not
recognise, compare with `HEAD` (`git status --short`, `git show HEAD:<path>`) and restore from it; never trust the
copy on disk.
