# Pre-migration baseline (2026-07-10)

- Last successful main pipeline before the plan: Woodpecker rustnzb repo 38,
  pipeline 204, commit `aad5563b03e9f99866730216e044fc4adce5516d`, about 567 seconds.
- Plan commit pipeline 209 failed after about 224 seconds. Several concurrent
  tasks expired in the agent queue and the workflow ended with a duplicate
  Docker network error. This is why the converged workflow sequences the
  heavyweight gates instead of starting every build at once.
- The pre-migration Forgejo `dev` reference resolved to
  `sha256:57840249a2812a622b075c1df3ba4c96d639c59c410af295a6b7b6f6d7d15e30`.
  Preserve it as the first production rollback candidate.
- The old pipeline downloaded sccache in each Rust step, installed Rust
  components at runtime, installed Playwright/browser packages in E2E, and
  installed GTK packages in desktop. It built the frontend both in a separate
  task and conditionally inside Docker.
- The host filesystem had 57 GiB free (97% used) before migration work. Every
  arm64 attempt must capture Buildx usage and filesystem high-water marks.
- Forgejo API, Forgejo git, and Woodpecker API authentication passed. The
  secondary GitHub PAT returned HTTP 401 and must be rotated before GHCR or a
  GitHub release can be validated; Forgejo work is unaffected.

Runtime metrics are written under `.ci-output/metrics` by image-build and cold
build tasks. Fill the cold/warm comparison in the final migration record from
the first successful production-runner pipelines; local timings are not a
substitute for runner metrics.

## Post-migration result (2026-07-10)

- First fully successful converged main pipeline: repo 38, pipeline 218,
  commit `2385c85fcad7981c08b0ae8b12725c05c3b89558`.
- The pipeline passed the complete deterministic suite, including 85
  Playwright tests, and published/smoke-tested/promoted the immutable amd64
  candidate.
- The candidate manifest digest was
  `sha256:de788ac1ed31d2ccca47ca0af6e0903fb0d4083eb9ce5f08b760663f4258d947`.
- Node B deployment used ops commit `404f604` and Komodo `personal-arr`.
  Runtime validation returned healthy, zero restarts, HTTP 200 for `/`, and
  `{"status":"ok"}` from `/api/health`.
- The runner reached 100% filesystem usage during the migration. Removing only
  reproducible checkout outputs (`target/` and `.ci-output/`) recovered about
  88 GiB; no application data or live container state was removed.
