# 2026-08-20: SignPath Code Signing (Windows)

## Decision

Added the CI plumbing to Authenticode-sign `TunnyDashboard.exe` via the
[SignPath Foundation OSS program](https://signpath.io/solutions/open-source-community),
following the pattern shown in
[`SignPath/github-actions-demo`](https://github.com/SignPath/github-actions-demo)
(the reference project for
[`signpath/github-action-submit-signing-request`](https://github.com/SignPath/github-action-submit-signing-request)).
This does not yet sign anything — the SignPath project must still be applied
for and approved, and its secrets/variables configured — but the workflow and
artifact configuration are ready to go the moment that happens.

Decisions made, with rejected alternatives:

- **Sign the bare exe, not the packaged zip.** `release.yml` bundles
  `TunnyDashboard.exe` with `lib_lightgbm.dll`, `README.md`, and `LICENSE`
  into the shipped `.zip`. Only the exe is signed — `lib_lightgbm.dll` is a
  third-party prebuilt binary this project doesn't own the source of, and
  signing it would misrepresent its provenance. Rejected: uploading the
  whole assembled zip to SignPath and using a `<zip-file>` config matching
  all four members — unnecessary complexity for files that don't need a
  signature, and it would tie the sign step to the packaging step instead of
  running right after the build, before packaging.
- **Artifact configuration is `<zip-file><pe-file path="TunnyDashboard.exe">`,
  not a bare `<pe-file>` root.** GitHub Actions artifacts are always zip
  archives in transit, and SignPath's own examples (the demo project above,
  and the [PoE-Overlay-Community-Fork
  config](https://github.com/PoE-Overlay-Community/PoE-Overlay-Community-Fork/blob/master/SignPath-Artifact-Configuration.xml))
  consistently wrap the artifact root in `<zip-file>` even for simple cases,
  so that convention was followed rather than guessing at an unwrapped-root
  syntax that isn't demonstrated anywhere found.
- **Signing gated to `refs/tags/v*` pushes on `Tunny-gh/Tunny-Dashboard`,
  not `workflow_dispatch`.** Manual dispatch runs are for testing the
  packaging pipeline and don't produce a GitHub Release, so they don't need
  a real signature and shouldn't spend SignPath's signing quota. This also
  means forks (which don't have the `SIGNPATH_*` secrets) never hit a step
  that would fail for lack of them. Rejected: mirroring the demo project's
  branch-based `release-signing` vs. `test-signing` policy split — this repo
  only ever produces "real" releases from tags, so a second policy would be
  unused complexity.
- **`project-slug`, `signing-policy-slug`, and the artifact-configuration
  slug are hardcoded placeholders** (`tunny-dashboard`, `release-signing`,
  `default`) rather than made configurable. These names are chosen when the
  SignPath project is created in their dashboard after OSS approval — this
  session has no SignPath account and could not create the project to learn
  the real values, so the workflow and `.signpath/artifact-configurations/default.xml`
  use names that read naturally and must be reconciled with (or renamed to
  match) whatever is actually configured in SignPath. See Open Items.
- **Source material**: the referenced Zenn article
  (`https://zenn.dev/shm_7ec/articles/signpath-oss-code-signing`) that
  prompted this work could not be fetched — `zenn.dev`, `docs.signpath.io`,
  and `about.signpath.io` are all blocked by this session's network egress
  policy. The above (workflow + artifact configuration) instead comes from
  cloning SignPath's own public GitHub repositories (`github-actions-demo`,
  `github-action-submit-signing-request`) and web search results describing
  the OSS application process and artifact-configuration syntax. The user
  later supplied the article as a PDF directly (see the second half of this
  note) — it turned out to be Part 1 of a series ("申請編" / "Application"),
  scoped only to *preparing and submitting* the SignPath Foundation
  application, not to GitHub Actions integration (that's a follow-up article,
  unpublished as of this writing, gated on the author's own application being
  approved). The workflow/artifact-configuration work above was therefore
  built ahead of what the source article itself covers, based on SignPath's
  own official examples rather than the article — still accurate, but
  premature relative to the article's scope. It was kept rather than reverted
  since it's inert until the secrets described below are configured.

## Addendum: application-prep documents (from the actual article)

The user later shared the article's content directly (Zenn blocks this
session's egress, so it could not be fetched independently). It documents the
*pre-application* checklist for HardwareVisualizer's own SignPath Foundation
submission, and recommends two governance documents be added to the
repository before applying, to make the review easier: a code signing policy
page, and a `CODE_OF_CONDUCT.md`.

Before adding them, both were checked against SignPath's own published terms
(<https://github.com/SignPath/Website-old/blob/v2/src/drafts/oss_policy.md>,
their actual OSS Code of Conduct — not just the article's paraphrase) rather
than taken at face value, per the user's explicit request not to follow the
source material uncritically:

- **`CODE_SIGNING_POLICY.md` is a genuine SignPath requirement**, not just
  the article author's preference: their terms mandate a "Code signing
  policy" section on the project's home page and download/release pages,
  with a specific attribution line ("Free code signing provided by
  SignPath.io, certificate by SignPath Foundation") and a specific privacy
  statement ("This program will not transfer any information to other
  networked systems unless specifically requested by the user or the person
  installing or operating it.") verbatim. It only matters once a certificate
  is actually in use, not for submitting the application form itself, but
  since it costs little to prepare now and directly supports the CI plumbing
  already in place, it was added at repo root.
- **`CODE_OF_CONDUCT.md` is not a SignPath requirement at all** — nothing in
  SignPath's terms mentions it. It's the article author's own general
  OSS-hygiene practice, unrelated to the signing application. This was
  surfaced to the user explicitly (with a recommendation to skip it for now,
  given the project's current single-maintainer scale) before adding it. The
  user initially asked for it anyway (Contributor Covenant v2.1, enforcement
  contact via GitHub Issues / direct contact with `@hrntsm`, no email
  published), but on reflection — since it has no bearing on SignPath
  approval — decided against adding it for now. Not created; can be revisited
  independently of code signing whenever the project actually wants an OSS
  conduct policy.
- `CODE_SIGNING_POLICY.md`'s content needed one fact only the user could
  supply: the maintainer's GitHub handle for the policy's
  Authors/Reviewers/Approvers roles (`hrntsm`).

## What changed

- `.github/workflows/release.yml`: added `actions: read` to top-level
  `permissions` (required by the SignPath action to look up and download its
  own job's artifact via the GitHub API); added three Windows-only,
  tag-push-only steps between "Resolve version" and "Assemble package
  (Windows)": upload the unsigned exe as a throwaway artifact, submit it to
  SignPath via `signpath/github-action-submit-signing-request@v2` and wait
  for completion, then overwrite `target/release/TunnyDashboard.exe` with
  the signed copy before packaging continues unchanged.
- `.signpath/artifact-configurations/default.xml`: new file, declares
  `TunnyDashboard.exe` as the single Authenticode-signable PE file in the
  uploaded artifact.
- `CONTRIBUTING.md`: new "Code signing (Windows)" subsection under
  "Releasing", documenting the required `SIGNPATH_API_TOKEN` secret and
  `SIGNPATH_ORGANIZATION_ID` variable, and pointing at this note; later
  reworded to say signing is "intended", not already active, and linked to
  `CODE_SIGNING_POLICY.md`.
- `CHANGELOG.md`: `[Unreleased]` entry noting the (currently inactive)
  infrastructure.
- `CODE_SIGNING_POLICY.md` (new, repo root): per-platform signing policy —
  Windows/SignPath status and required attribution/privacy statements, macOS
  ad-hoc signing, and a note that Linux artifacts aren't built at all.
  Required by SignPath's OSS terms once a certificate is in use (see
  Addendum above).
- `README.md`: added a "Code signing policy" section linking to
  `CODE_SIGNING_POLICY.md`, satisfying SignPath's "must be specified on the
  project's home page" requirement.
- `.github/workflows/release.yml`: GitHub Release body now links to
  `CODE_SIGNING_POLICY.md` too (SignPath's "download/release pages"
  requirement).

## Open Items

- **Not yet functional.** No SignPath project exists for this repo yet. To
  activate:
  1. Apply at <https://signpath.io/solutions/open-source-community> (OSS
     eligibility: open-source license, public repo, no malware — this repo
     already qualifies).
  2. Once approved, create a SignPath project. If its slug ends up different
     from `tunny-dashboard`, or the signing policy isn't named
     `release-signing`, or the artifact configuration isn't named `default`,
     update the corresponding values in `release.yml` and rename
     `.signpath/artifact-configurations/default.xml` to match — the file's
     content doesn't need to change, only its slug (= file name).
  3. Create an API token for a user with the `Submitter` role on the
     `release-signing` policy; add it as the repository secret
     `SIGNPATH_API_TOKEN`.
  4. Add the organization ID (GUID, shown in the SignPath dashboard) as the
     repository variable `SIGNPATH_ORGANIZATION_ID`.
  5. Push a `v*` tag (or re-run `release.yml` after a real tag push) and
     confirm the "Sign executable (Windows)" step succeeds and the shipped
     `.zip` contains a signed exe (`Get-AuthenticodeSignature` on Windows,
     or `osslsigncode verify`).
- **Windows SmartScreen**: even once signed, a fresh certificate has no
  download-reputation history, so SmartScreen warnings may not disappear
  immediately. Worth revisiting the README's installation instructions (it
  currently only documents the macOS Gatekeeper workaround) once signing is
  live and the actual user experience is known.
- **Untested.** No SignPath account was available in this session, so the
  signing step has not been exercised against the real API — only checked
  against SignPath's own reference examples for shape and consistency.
- **Application itself is still manual.** This session prepared the
  repository (CI plumbing, artifact configuration, `CODE_SIGNING_POLICY.md`)
  but did not — and cannot — submit the actual SignPath Foundation
  application form at <https://signpath.org/apply.html> (Project/Repository
  URL, License, Download/Release URL, Project description); that's a step
  only the user/maintainer can take, since it requires an account and a
  contact email able to receive follow-up correspondence.
- **`CODE_OF_CONDUCT.md` intentionally not added** — see Addendum above;
  it doesn't affect SignPath approval and the user opted out of it for now.
