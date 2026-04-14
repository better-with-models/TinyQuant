# syntax=docker/dockerfile:1.7
#
# TinyQuant CLI container image — Phase 22.D, Step 18.
#
# Reproducibility contract (see
# docs/plans/rust/phase-22-pyo3-cabi-release.md §Container reproducibility):
#
#   1. Base images are pinned by **digest** (sha256), not by tag alone.
#      The sentinels below are resolved to real digests by the
#      `rust-release.yml` workflow's `resolve-base-digests` step before
#      `docker buildx build` runs. The sentinels exist so that this
#      file is self-documenting and builds on an operator's laptop can
#      optionally supply `--build-arg` overrides.
#
#      During local / dry-run builds the placeholder `@sha256:dry-run`
#      value is accepted — Docker resolves the tag on pull. CI enforces
#      real digests via an awk guard in `rust-release.yml`.
#
#   2. `SOURCE_DATE_EPOCH` is exported by the workflow from the tagged
#      commit's author date (`git log -1 --format=%ct`) and passed into
#      this Dockerfile as `--build-arg`. Cargo and Debian tooling
#      honour it so the release binary and OCI layer mtimes are
#      timestamp-stable.
#
#   3. OCI manifest timestamp normalization. The `rust-release.yml`
#      workflow sets `SOURCE_DATE_EPOCH` as a step-level env var on
#      every `docker/build-push-action@v6` invocation and pairs it
#      with `outputs: …,rewrite-timestamp=true`. BuildKit then
#      rewrites layer mtimes in the resulting OCI manifest to the
#      epoch value. This Dockerfile carries only the build-time half
#      (the `SOURCE_DATE_EPOCH` build-arg + env); the post-build
#      rewrite lives in the workflow.
#
#   4. Deterministic release profile — `codegen-units = 1`,
#      `lto = "fat"`, `strip = true`, `debug = false` — is set in
#      `rust/Cargo.toml [profile.release]`. Bytewise reproducibility
#      is asserted by the workflow's `container-reproducibility` job.
#
# Runtime: non-root via distroless `nonroot` user. Default CMD runs
# `tinyquant info` so a bare `docker run ghcr.io/...` smoke-tests the
# binary without requiring subcommand args.
#
# Feature set: the builder below compiles with `--features tracing-json`
# on top of the CLI default feature set (`jemalloc`, `rayon`, `simd`,
# `mmap`, `progress`). The extra feature flips `tracing-subscriber`
# into JSON mode so `docker logs ghcr.io/...` emits structured records
# consumable by `jq` / downstream log pipelines. See
# docs/plans/rust/phase-22-pyo3-cabi-release.md §CLI feature flag matrix.

# --- Stage 1: builder -------------------------------------------------------
ARG RUST_BUILDER_DIGEST=sha256:REPLACE_WITH_PINNED_DIGEST_RUST_1_81_BOOKWORM
ARG RUNTIME_DIGEST=sha256:REPLACE_WITH_PINNED_DIGEST_DISTROLESS_CC_DEBIAN12_NONROOT

FROM rust:1.81-bookworm@${RUST_BUILDER_DIGEST} AS builder

# Timestamp-stable builds: the workflow passes the tag commit's ctime.
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}

# Build context is the repo root; only the `rust/` subtree matters
# (see `rust/.dockerignore`). The rest of the tree is filtered out.
WORKDIR /build
COPY rust /build/rust

WORKDIR /build/rust
RUN cargo build \
        -p tinyquant-cli \
        --release \
        --locked \
        --features tracing-json

# Sanity-check the binary exists at the expected path (fails loudly if
# cargo is reconfigured to a workspace-scoped target-dir). Also
# doubles as a smoke check on the builder stage — `tinyquant --version`
# must link and run before we copy it into the runtime image.
RUN /build/rust/target/release/tinyquant --version

# --- Stage 2: runtime -------------------------------------------------------
FROM gcr.io/distroless/cc-debian12:nonroot@${RUNTIME_DIGEST} AS runtime

COPY --from=builder /build/rust/target/release/tinyquant /usr/local/bin/tinyquant

USER nonroot
ENTRYPOINT ["/usr/local/bin/tinyquant"]
CMD ["info"]
