use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use sesh_integration_tests::Runner;
use sesh_integration_tests::Script;
use tokio::runtime::Builder;

const ROOT: &str = "tests/cases";

fn test(path: &Path) -> datatest_stable::Result<()> {
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

        runner
            .run(&mut output, &script)
            .await
            .context("failed to write integration test output")
    })?;

    let mut snapshots = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    snapshots.extend(["tests", "snapshots"]);

    let snapshot_name = path
        .strip_prefix(ROOT)?
        .to_str()
        .context("invalid test name")?
        .replace('/', "__");

    let snapshot_name = format!("test__{snapshot_name}");
    insta::with_settings!({ prepend_module_to_snapshot => false, snapshot_path => snapshots }, {
        insta::assert_snapshot!(snapshot_name, output);
    });

    Ok(())
}

datatest_stable::harness! {
    { test = test, root = ROOT, pattern = r".*\.md$" },
}
