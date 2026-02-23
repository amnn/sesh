use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use tokio::runtime::Builder;

use sesh_integration_tests::Runner;
use sesh_integration_tests::Script;

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
            .run(&mut output, &script)
            .await
            .context("failed to write integration test output")
    })?;

    let snapshot_name = path
        .strip_prefix("tests/cases")?
        .to_str()
        .context("invalid test name")?
        .replace('/', "__");

    insta::assert_snapshot!(snapshot_name, output);
    Ok(())
}

datatest_stable::harness! {
    { test = test, root = "tests/cases", pattern = r".*\.md$" },
}
