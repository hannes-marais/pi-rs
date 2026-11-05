2026-11-05

# Windows OS metadata portability

Owner: Codex; EMgine VPM vm-1d9a. Outcome: make Pi's Node-compatible os metadata compile and report native Windows values while preserving Unix behavior. Architecture principles 3, 6, 13, 21, 24, 27, 30, 35, 37 govern truthful target evidence, declared dependencies and platform lowering. ACC: /home/oumy/work/emgine/config/man/anvil.md. ToQ: metadata is obtained from the target OS API and never fabricated from the build host; ToT: use the locked windows-sys API already present in the workspace dependency graph and retain the existing Unix syscall implementation.

Files: crates/pi-jsrt/Cargo.toml and src/ops/process.rs. Risks: Windows version APIs may report compatibility-manifest-filtered values; tests must run on native OptiPlex1 as well as compile. Validation: locked offline Windows-target check of pi-jsrt, Linux pi-jsrt check, and native OptiPlex1 metadata smoke test.
