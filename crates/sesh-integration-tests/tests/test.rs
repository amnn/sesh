use std::path::Path;

fn test(path: &Path) -> datatest_stable::Result<()> {
    let result = sesh_integration_tests::runner::run_case_file(path)?;
    insta::assert_snapshot!(result.snapshot_name, result.transcript);
    Ok(())
}

datatest_stable::harness! {
    { test = test, root = "tests/cases", pattern = r".*\.md$" },
}
