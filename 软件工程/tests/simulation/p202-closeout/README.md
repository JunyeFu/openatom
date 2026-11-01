# P2-02 transport closeout tests

Install this complete directory at `tests/simulation/p202-closeout/` in the software-engineering repository. Keep all 21 `fixtures/*.json` files. Dependencies and the unchanged independent acceptance test resolve to the repository's existing `packages/` using manifest-relative paths. No worker directory is required.

From the repository root, after disk space is restored:

```powershell
powershell -NoProfile -File tests/simulation/p202-closeout/run-tests.ps1
```

Or run ordinary Cargo without an evidence-directory variable:

```powershell
cargo test --manifest-path tests/simulation/p202-closeout/Cargo.toml --locked --offline --target-dir tests/simulation/p202-closeout/.work/target -j 1 -- --nocapture
```

The wrapper additionally keeps process-local TEMP/TMP under `.work/temp`. Evidence is always written to this crate's `.work/delivery/`; valid cases save and move projects and launch `p202-reopen` in fresh OS processes. This standalone workspace does not require editing the repository-root workspace members or lock file.

The portable candidate has only static path/manifest/input verification in this revision. Earlier 17 Host/Engine runs and 17 fresh-process checks used the original worker layout and are retained separately; they are not runtime proof for this changed candidate. Rerun before declaring the portable candidate accepted.
