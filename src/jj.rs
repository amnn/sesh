use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::Context as _;

/// Discover valid jj repositories from directories matching `globs`.
pub fn repos(globs: &[String]) -> anyhow::Result<BTreeSet<PathBuf>> {
    let mut repos = BTreeSet::new();
    for pattern in globs {
        for path in glob::glob(pattern).with_context(|| format!("invald glob: '{pattern}'"))? {
            if let Ok(path) = path
                && path.join(".jj").is_dir()
            {
                repos.insert(path);
            }
        }
    }

    Ok(repos)
}
