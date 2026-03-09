// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::Context as _;
use sesh_integration_tests::Runner;
use sesh_integration_tests::Script;
use telemetry_subscribers::TelemetryConfig;
use telemetry_subscribers::TelemetryGuards;
use tokio::runtime::Builder;

const ROOT: &str = "tests/cases";
static TRACE: LazyLock<TelemetryGuards> = LazyLock::new(|| {
    let (guards, _handle) = TelemetryConfig::new("sesh").with_env().init();

    guards
});

fn test(path: &Path) -> datatest_stable::Result<()> {
    LazyLock::force(&TRACE);

    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let input = std::fs::read_to_string(path)?;
    let script = Script::parse(&input);

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to construct async runtime for integration runner")?;

    let mut output = String::new();
    runtime.block_on(async {
        let mut runner = Runner::new(&tmp).await?;
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

    let mut snapshots = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    snapshots.extend(["tests", "snapshots"]);

    let snapshot_components: anyhow::Result<Vec<_>> = path
        .strip_prefix(ROOT)?
        .components()
        .map(|component| component.as_os_str().to_str().context("invalid test name"))
        .collect();

    let snapshot_name = snapshot_components?.join("__");
    let snapshot_name = format!("test__{snapshot_name}");
    insta::with_settings!({ prepend_module_to_snapshot => false, snapshot_path => snapshots }, {
        insta::assert_snapshot!(snapshot_name, output);
    });

    Ok(())
}

datatest_stable::harness! {
    { test = test, root = ROOT, pattern = r".*\.md$" },
}
