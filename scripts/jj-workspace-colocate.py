#!/usr/bin/env python3
# Copyright (c) Ashok Menon
# SPDX-License-Identifier: Apache-2.0

"""Attach a Git linked worktree to an existing jj workspace.

Released versions of jj can create secondary workspaces for a colocated Git
repository, but those secondary workspaces only contain `.jj/` metadata. Git
commands run from them therefore fail repository discovery. This helper creates
the missing Git linked-worktree metadata and can later realign that metadata with
jj @-.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path


@dataclass(frozen=True)
class CommandResult:
    """Captured output from a subprocess invocation."""

    returncode: int
    stderr: str
    stdout: str


@dataclass(frozen=True)
class GitDirs:
    """Git metadata directories for a checkout."""

    common_dir: Path
    git_dir: Path


@dataclass(frozen=True)
class Workspace:
    """A jj workspace discovered from the current command context."""

    default_root: Path
    root: Path
    name: str | None


@dataclass(frozen=True)
class SyncResult:
    """Summary of a sync operation."""

    commit: str
    explanation: str
    title: str


BOLD = "1"
BRIGHT_YELLOW = "93"
DIM = "2"
GREEN = "32"
LABEL_WIDTH = 18
RED = "31"
WRAP_WIDTH = 88


def command_error(
    args: list[str], result: subprocess.CompletedProcess[str]
) -> RuntimeError:
    """Build a readable exception for a failed command."""

    command = " ".join(args)
    stderr = result.stderr.strip()
    stdout = result.stdout.strip()

    if stderr:
        return RuntimeError(f"command failed: {command}\n{stderr}")
    elif stdout:
        return RuntimeError(f"command failed: {command}\n{stdout}")
    else:
        return RuntimeError(f"command failed: {command}")


def run(
    args: list[str],
    *,
    check: bool = True,
    cwd: Path | None = None,
    stdin: str | None = None,
) -> CommandResult:
    """Run a subprocess and return captured text output."""

    result = subprocess.run(
        args,
        check=False,
        cwd=cwd,
        input=stdin,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    )

    if check and result.returncode != 0:
        raise command_error(args, result)

    return CommandResult(
        returncode=result.returncode,
        stderr=result.stderr,
        stdout=result.stdout,
    )


def emit_explanation(text: str) -> None:
    """Print dim explanatory text."""

    for line in textwrap.wrap(text, width=WRAP_WIDTH):
        print(f"  {styled(line, DIM)}")


def emit_fix(command: str, explanation: str) -> None:
    """Print a suggested repair command and explain what it does."""
    emit_status_block("fix", command, BRIGHT_YELLOW, explanation)


def emit_group(values: list[tuple[str, object]], explanation: str) -> None:
    """Print related key/value lines followed by one explanation."""

    for label, value in values:
        emit_value(label, value)

    print()
    emit_explanation(explanation)
    print()


def emit_problem(message: str, explanation: str) -> None:
    """Print a problem header and explanation."""
    emit_status_block("problem", message, RED, explanation)


def emit_status(kind: str, message: str, style: str) -> None:
    """Print a colored status header."""

    line = f"{kind}:".ljust(LABEL_WIDTH) + f" {message}"
    print(styled(line, style))


def emit_status_block(kind: str, message: str, style: str, explanation: str) -> None:
    """Print a colored status header followed by one explanation."""

    emit_status(kind, message, style)
    print()
    emit_explanation(explanation)
    print()


def emit_value(label: str, value: object) -> None:
    """Print a bold label and aligned value."""

    prefix = f"{label}:".ljust(LABEL_WIDTH)
    print(f"{styled(prefix, BOLD)} {value}")


def emit_ok(message: str, explanation: str) -> None:
    """Print a success header and explanation."""
    emit_status_block("ok", message, GREEN, explanation)


def styled(text: str, style: str) -> str:
    """Apply ANSI styling when stdout supports it."""

    if not sys.stdout.isatty() or os.environ.get("NO_COLOR"):
        return text
    return f"\033[{style}m{text}\033[0m"


def format_sparse_patterns(patterns: list[str] | None) -> str:
    """Format sparse patterns for human-readable output."""

    if patterns is None:
        return "full checkout"
    return ", ".join(patterns)


def backup_file(path: Path) -> Path:
    """Move an existing file aside and return the backup path."""

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    destination = path.with_name(f"{path.name}.backup-{timestamp}")
    counter = 1
    while destination.exists():
        destination = path.with_name(f"{path.name}.backup-{timestamp}.{counter}")
        counter += 1
    path.rename(destination)
    return destination


def ensure_jj_ignored(workspace_root: Path) -> None:
    """Ensure Git ignores jj's private metadata directory."""

    ignore_path = workspace_root / ".jj" / ".gitignore"
    ignore_path.parent.mkdir(parents=True, exist_ok=True)

    if not ignore_path.exists():
        ignore_path.write_text("*\n", encoding="utf-8")
        return

    content = ignore_path.read_text(encoding="utf-8")
    if "*" in content.splitlines():
        return

    separator = "" if content.endswith("\n") else "\n"
    with ignore_path.open("a", encoding="utf-8") as ignore_file:
        ignore_file.write(f"{separator}*\n")


def git_dirs(path: Path) -> GitDirs:
    """Return the Git directory and common directory for `path`."""

    git_dir = run(
        ["git", "-C", str(path), "rev-parse", "--path-format=absolute", "--git-dir"]
    )

    common_dir = run(
        [
            "git",
            "-C",
            str(path),
            "rev-parse",
            "--path-format=absolute",
            "--git-common-dir",
        ]
    )

    return GitDirs(
        common_dir=Path(common_dir.stdout.strip()).resolve(),
        git_dir=Path(git_dir.stdout.strip()).resolve(),
    )


def git_head(path: Path) -> str:
    """Return the Git HEAD commit for `path`."""

    return run(["git", "-C", str(path), "rev-parse", "HEAD"]).stdout.strip()


def git_set_sparse_patterns(path: Path, patterns: list[str]) -> None:
    """Configure Git sparse-checkout for `path` to use `patterns`."""

    sparse_input = "\n".join(patterns) + "\n"
    run(["git", "-C", str(path), "sparse-checkout", "init", "--no-cone"])
    run(
        [
            "git",
            "-C",
            str(path),
            "sparse-checkout",
            "set",
            "--no-cone",
            "--stdin",
        ],
        stdin=sparse_input,
    )


def git_sparse_patterns(path: Path) -> list[str] | None:
    """Return Git sparse-checkout patterns, or None when sparse checkout is off."""

    result = run(
        ["git", "-C", str(path), "sparse-checkout", "list"],
        check=False,
    )
    if result.returncode != 0:
        return None
    patterns = [line.strip() for line in result.stdout.splitlines() if line.strip()]
    return patterns or None


def git_toplevel(path: Path) -> Path | None:
    """Return Git's top-level worktree for `path`, or None if discovery fails."""

    result = run(
        ["git", "-C", str(path), "rev-parse", "--show-toplevel"],
        check=False,
    )

    if not result.stdout.strip():
        return None

    return Path(result.stdout.strip()).resolve()


def is_git_linked_worktree(path: Path) -> bool:
    """Return whether `path` uses a linked-worktree Git directory."""

    if not (path / ".git").is_file():
        return False
    dirs = git_dirs(path)
    return dirs.git_dir != dirs.common_dir


def jj_parent_commit(path: Path) -> str:
    """Return the single Git commit that should be Git HEAD for a jj workspace."""

    output = run(
        [
            "jj",
            "log",
            "--ignore-working-copy",
            "-r",
            "@-",
            "--no-graph",
            "-T",
            "commit_id ++ '\n'",
        ],
        cwd=path,
    ).stdout
    commits = [line.strip() for line in output.splitlines() if line.strip()]
    if len(commits) != 1:
        raise RuntimeError(
            "expected exactly one jj @- commit; "
            "pass --revision when linking merge or root workspaces"
        )
    commit = commits[0]
    if not commit.strip("0"):
        raise RuntimeError("jj @- is root(), which Git cannot check out as HEAD")
    return commit


def jj_sparse_patterns(path: Path) -> list[str] | None:
    """Return jj sparse patterns for a workspace, or None for a full checkout."""

    output = run(["jj", "sparse", "list", "--ignore-working-copy"], cwd=path).stdout
    patterns = [line.strip() for line in output.splitlines() if line.strip()]
    if not patterns or patterns == ["."]:
        return None
    return patterns


def jj_workspace_list(path: Path) -> list[tuple[str, Path]]:
    """Return `(name, root)` entries from jj's workspace list."""

    output = run(
        [
            "jj",
            "workspace",
            "list",
            "--ignore-working-copy",
            "-T",
            "name ++ '\t' ++ root ++ '\n'",
        ],
        cwd=path,
    ).stdout
    workspaces: list[tuple[str, Path]] = []
    for line in output.splitlines():
        if "\t" not in line:
            continue
        name, root = line.split("\t", 1)
        workspaces.append((name, Path(root).resolve()))
    return workspaces


def jj_workspace_root(path: Path) -> Path:
    """Return the jj workspace root containing `path`."""

    output = run(["jj", "workspace", "root", "--ignore-working-copy"], cwd=path).stdout
    return Path(output.strip()).resolve()


def resolve_workspace(path: Path, default_workspace: Path | None) -> Workspace:
    """Discover the current jj workspace and the default colocated Git workspace."""

    root = jj_workspace_root(path)
    workspaces = jj_workspace_list(root)
    name = next(
        (
            workspace_name
            for workspace_name, workspace_root in workspaces
            if workspace_root == root
        ),
        None,
    )

    if default_workspace is not None:
        default_root = default_workspace.resolve()
    else:
        default_candidates = [
            workspace_root
            for _, workspace_root in workspaces
            if (workspace_root / ".jj" / "repo").is_dir()
        ]
        default_root = next(
            (
                candidate
                for candidate in default_candidates
                if git_toplevel(candidate) == candidate
            ),
            None,
        )
        if default_root is None and git_toplevel(root) == root:
            default_root = root
        if default_root is None:
            raise RuntimeError(
                "could not find the default colocated Git workspace; "
                "pass --default-workspace"
            )

    if git_toplevel(default_root) != default_root:
        raise RuntimeError(f"default workspace is not a Git worktree: {default_root}")

    return Workspace(default_root=default_root, name=name, root=root)


def sync_git_sparse(workspace_root: Path) -> None:
    """Configure Git sparse-checkout to match jj sparse patterns."""

    patterns = jj_sparse_patterns(workspace_root)
    if patterns is None:
        return

    git_set_sparse_patterns(workspace_root, patterns)


def sync_worktree(
    workspace_root: Path, revision: str | None, *, sync_sparse: bool
) -> str:
    """Move Git HEAD and the Git index to jj @-."""

    commit = revision or jj_parent_commit(workspace_root)

    # Make sure the commit is known to `git` before trying to reset to it.
    run(["git", "-C", str(workspace_root), "cat-file", "-e", f"{commit}^{{commit}}"])

    if sync_sparse:
        sync_git_sparse(workspace_root)

    run(["git", "-C", str(workspace_root), "reset", "--mixed", commit])
    ensure_jj_ignored(workspace_root)
    return commit


def sync_workspace(
    workspace: Workspace,
    revision: str | None,
    *,
    replace_existing: bool,
    sync_sparse: bool,
) -> SyncResult:
    """Create or synchronize linked-worktree metadata for an existing jj workspace."""

    dot_git = workspace.root / ".git"
    if is_git_linked_worktree(workspace.root):
        commit = sync_worktree(workspace.root, revision, sync_sparse=sync_sparse)
        return SyncResult(
            commit=commit,
            explanation=f"Synced git HEAD/index to jj @- {commit}.",
            title="synced git HEAD/index",
        )

    replace_dot_git = dot_git.exists()
    if replace_dot_git and (not replace_existing or not dot_git.is_file()):
        raise RuntimeError(
            f"{dot_git} already exists; use sync for an existing linked worktree "
            "or --replace-existing for an unsafe gitdir pointer"
        )

    commit = revision or jj_parent_commit(workspace.root)
    run(
        [
            "git",
            "-C",
            str(workspace.default_root),
            "cat-file",
            "-e",
            f"{commit}^{{commit}}",
        ]
    )

    temp_parent = workspace.root.parent
    temp_prefix = f".jj-git-worktree-{workspace.name or workspace.root.name}."
    temp_path = Path(tempfile.mkdtemp(dir=temp_parent, prefix=temp_prefix))
    backup_path: Path | None = None
    try:
        run(["git", "-C", str(workspace.default_root), "worktree", "prune"])
        run(
            [
                "git",
                "-C",
                str(workspace.default_root),
                "worktree",
                "add",
                "--detach",
                "--no-checkout",
                str(temp_path),
                commit,
            ]
        )
        if sync_sparse:
            patterns = jj_sparse_patterns(workspace.root)
            if patterns is not None:
                git_set_sparse_patterns(temp_path, patterns)
        run(["git", "-C", str(temp_path), "checkout", "--detach", commit])
        if replace_dot_git:
            backup_path = backup_file(dot_git)
            emit_group(
                [("backup", f"{dot_git} -> {backup_path}")],
                "The previous .git file is preserved so the replacement can be "
                "inspected or restored manually if needed.",
            )
        (temp_path / ".git").rename(dot_git)
        ensure_jj_ignored(workspace.root)
        run(
            [
                "git",
                "-C",
                str(workspace.default_root),
                "worktree",
                "repair",
                str(workspace.root),
            ]
        )
        sync_worktree(workspace.root, commit, sync_sparse=sync_sparse)
    except Exception:
        if backup_path is not None and backup_path.exists() and not dot_git.exists():
            backup_path.rename(dot_git)
        raise
    finally:
        shutil.rmtree(temp_path, ignore_errors=True)

    return SyncResult(
        commit=commit,
        explanation=f"Created linked worktree metadata using jj @- {commit}.",
        title="linked git worktree metadata",
    )


def doctor(workspace: Workspace) -> int:
    """Inspect whether the current jj workspace is backed by a safe Git worktree."""

    emit_group(
        [
            ("jj workspace", workspace.root),
            ("default workspace", workspace.default_root),
        ],
        "The workspace being checked and the workspace that owns the colocated "
        "git repository. Git tools should discover the workspace as their "
        "worktree root.",
    )

    git_root = git_toplevel(workspace.root)
    if git_root is None:
        emit_problem(
            "Git does not detect a repository from this workspace",
            "Git-based tools will behave as if this checkout is unversioned, even "
            "though jj can use it as a workspace.",
        )
        emit_fix(
            "scripts/jj-workspace-colocate.py sync",
            "Creates linked git worktree metadata for this jj workspace without "
            "rewriting the files in the working tree.",
        )
        return 1

    dirs = git_dirs(workspace.root)
    emit_group(
        [
            ("git toplevel", git_root),
            ("git dir", dirs.git_dir),
            ("git common dir", dirs.common_dir),
        ],
        "Git's toplevel should match jj's workspace root. A different path "
        "means git is looking at another checkout instead of this workspace. "
        "A secondary workspace should use a worktree-specific git directory "
        "different from the common directory that contains the shared objects "
        "and refs.",
    )

    if git_root != workspace.root:
        emit_problem(
            "Git discovers a different worktree root",
            "Commands such as git status, git stash, and cargo fix would operate "
            "relative to the wrong checkout.",
        )
        emit_fix(
            "cd into the jj workspace root, then run "
            "scripts/jj-workspace-colocate.py sync",
            "Re-run discovery from the workspace that should receive the git "
            "worktree metadata.",
        )
        return 1

    if dirs.git_dir == dirs.common_dir and workspace.root != workspace.default_root:
        emit_problem(
            "this workspace shares the default .git index",
            "A direct .git pointer to the default workspace makes git stash, git "
            "reset, and other git writes mutate shared state for the wrong "
            "checkout.",
        )
        emit_fix(
            "scripts/jj-workspace-colocate.py sync --replace-existing",
            "Backs up the existing .git file and replaces it with a proper linked "
            "worktree pointer that has an independent HEAD and index.",
        )
        return 1

    jj_parent = jj_parent_commit(workspace.root)
    head = git_head(workspace.root)
    emit_group(
        [
            ("jj @-", jj_parent),
            ("git HEAD", head),
        ],
        "Git's HEAD should match jj's parent commit.",
    )

    if head != jj_parent:
        emit_problem(
            "Git HEAD/index do not match jj @-",
            "Git tools may show diffs against an old base commit or stash changes "
            "relative to the wrong revision.",
        )
        emit_fix(
            "scripts/jj-workspace-colocate.py sync",
            "Runs git reset --mixed to jj @- and refreshes Git sparse-checkout "
            "metadata without changing working-tree file contents.",
        )
        return 1

    jj_patterns = jj_sparse_patterns(workspace.root)
    git_patterns = git_sparse_patterns(workspace.root)
    emit_group(
        [
            ("jj sparse", format_sparse_patterns(jj_patterns)),
            ("git sparse", format_sparse_patterns(git_patterns)),
        ],
        "Both VCS's should agree on sparsity.",
    )

    if git_patterns != jj_patterns:
        emit_problem(
            "Git sparse-checkout does not match jj sparse patterns",
            "Git may report noisy deletions or ignore files that jj expects to "
            "be part of the workspace.",
        )
        emit_fix(
            "scripts/jj-workspace-colocate.py sync",
            "Rewrites git sparse-checkout metadata from jj's sparse patterns and "
            "resets the Git index to jj @-.",
        )
        return 1

    jj_status = run(
        ["git", "-C", str(workspace.root), "status", "--short", "--", ".jj"],
        check=False,
    ).stdout.strip()
    emit_group(
        [("git .jj status", jj_status or "ignored")],
        "Git must ignore jj's private metadata so commands such as git stash -u "
        "or git clean do not remove the workspace state.",
    )

    if jj_status:
        emit_problem(
            "Git can see .jj metadata",
            "The workspace is at risk because git can treat jj's metadata as "
            "untracked files.",
        )
        emit_fix(
            "scripts/jj-workspace-colocate.py sync",
            "Writes .jj/.gitignore and refreshes git metadata so .jj stays out "
            "of git status output.",
        )
        return 1

    emit_ok(
        "workspace has linked Git worktree metadata",
        "Git discovery, HEAD/index, sparse-checkout, and jj metadata visibility "
        "all match the current jj workspace context.",
    )
    return 0


def add_common_arguments(parser: argparse.ArgumentParser) -> None:
    """Add common workspace discovery arguments to a subcommand parser."""

    parser.add_argument(
        "--workspace",
        type=Path,
        default=Path.cwd(),
        help="path inside the jj workspace to inspect (default: current directory)",
    )

    parser.add_argument(
        "--default-workspace",
        type=Path,
        help=(
            "default colocated Git workspace, if it cannot be discovered from jj "
            "metadata"
        ),
    )


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""

    parser = argparse.ArgumentParser(
        description="Attach or sync git linked-worktree metadata for a jj workspace.",
    )

    subcommands = parser.add_subparsers(dest="command", required=True)

    doctor_parser = subcommands.add_parser("doctor", help="check the current workspace")
    add_common_arguments(doctor_parser)

    sync_parser = subcommands.add_parser(
        "sync",
        help="create or sync git worktree metadata",
    )
    add_common_arguments(sync_parser)

    sync_parser.add_argument(
        "--replace-existing",
        action="store_true",
        help=(
            "back up and replace an existing .git file, such as a direct gitdir "
            "pointer"
        ),
    )

    sync_parser.add_argument(
        "--revision",
        help="git commit to use as HEAD instead of jj @-",
    )

    sync_parser.add_argument(
        "--no-sync-sparse",
        action="store_true",
        help="do not configure git sparse-checkout from jj sparse patterns",
    )

    return parser


def main() -> int:
    """Run the requested subcommand."""

    args = build_parser().parse_args()
    try:
        workspace = resolve_workspace(args.workspace.resolve(), args.default_workspace)

        if args.command == "doctor":
            return doctor(workspace)

        if args.command == "sync":
            result = sync_workspace(
                workspace,
                args.revision,
                replace_existing=args.replace_existing,
                sync_sparse=not args.no_sync_sparse,
            )
            emit_ok(result.title, result.explanation)
            return doctor(workspace)

    except RuntimeError as error:
        print(f"{styled('error:', RED)} {error}", file=sys.stderr)
        return 1

    print(f"{styled('error:', RED)} unknown command {args.command}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
