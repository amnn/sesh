// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Runs markdown-driven UI snapshot cases for the `sesh` binary.

use std::path::Path;
use std::sync::LazyLock;

use anyhow::Context as _;
use sesh_integration_tests::Runner;
use sesh_integration_tests::Script;
use sesh_integration_tests::remove_stale_snapshots;
use sesh_integration_tests::remove_test_svg_snapshots;
use telemetry_subscribers::TelemetryConfig;
use telemetry_subscribers::TelemetryGuards;
use tokio::runtime::Builder;

const ROOT: &str = "tests/cases";
static TRACE: LazyLock<TelemetryGuards> = LazyLock::new(|| {
    let (guards, _handle) = TelemetryConfig::new("sesh").with_env().init();

    guards
});

/// Executes one markdown case and snapshots the rendered terminal transcript.
fn test(path: &Path) -> datatest_stable::Result<()> {
    LazyLock::force(&TRACE);

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let case = manifest_dir.join(path);
    remove_stale_snapshots(&manifest_dir.join(ROOT))?;

    let input = std::fs::read_to_string(&case)?;
    let script = Script::parse(&input);

    let mut snapshot = case.clone();
    snapshot.add_extension("snap");
    remove_test_svg_snapshots(&snapshot)?;

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to construct async runtime for integration runner")?;

    let mut output = String::new();
    runtime.block_on(async {
        let mut runner = Runner::new(env!("CARGO_MANIFEST_DIR"), snapshot).await?;
        runner
            .bin(env!("CARGO_BIN_EXE_sesh"))
            .await
            .context("failed to add sesh binary to runner environment")?;

        let res = runner
            .run(&mut output, &script)
            .await
            .context("failed to write integration test output");

        runner.shutdown().await.ok();
        res
    })?;

    let snapshot_name = case
        .file_name()
        .and_then(|name| name.to_str())
        .context("invalid snapshot name")?
        .to_owned();

    let snapshot_path = case
        .parent()
        .context("test case must have a parent directory")?;

    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_path(snapshot_path);
    settings.set_input_file(case);
    settings.bind(|| insta::assert_snapshot!(snapshot_name, output));

    Ok(())
}

datatest_stable::harness! {
    { test = test, root = ROOT, pattern = r".*\.md$" },
}
