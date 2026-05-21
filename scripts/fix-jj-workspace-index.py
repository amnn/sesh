#!/usr/bin/env python3
# Copyright (c) Ashok Menon
# SPDX-License-Identifier: Apache-2.0

"""Rebuild jj's workspace path index for legacy repositories.

The script writes `.jj/repo/workspace_store/index` in the repository rooted at
`--repo` (or the current directory). Existing index files are backed up next to
the original before replacement.
"""

from __future__ import annotations

import argparse
import os
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path


def varint(value: int) -> bytes:
    """Encode a protobuf varint."""

    encoded = bytearray()
    while value >= 0x80:
        encoded.append((value & 0x7F) | 0x80)
        value >>= 7
    encoded.append(value)
    return bytes(encoded)


def field_string(number: int, value: str) -> bytes:
    """Encode a protobuf length-delimited string field."""

    data = value.encode()
    return bytes([(number << 3) | 2]) + varint(len(data)) + data


def parse_workspace(spec: str) -> tuple[str, Path]:
    """Parse a `name=path` workspace argument."""

    if "=" not in spec:
        raise argparse.ArgumentTypeError(
            "expected WORKSPACE in the form name=/path/to/checkout"
        )

    name, root = spec.split("=", 1)
    if not name:
        raise argparse.ArgumentTypeError("workspace name must not be empty")
    if not root:
        raise argparse.ArgumentTypeError(f"workspace '{name}' path must not be empty")

    return name, Path(root)


def backup(path: Path) -> Path | None:
    """Back up an existing index before replacing it."""

    if not path.exists():
        return None

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    destination = path.with_name(f"{path.name}.backup-{timestamp}")
    counter = 1
    while destination.exists():
        destination = path.with_name(f"{path.name}.backup-{timestamp}.{counter}")
        counter += 1

    shutil.copy2(path, destination)
    return destination


def build_index(repo_store: Path, workspaces: list[tuple[str, Path]]) -> bytes:
    """Build the workspace index protobuf payload."""

    index = bytearray()
    for name, root in workspaces:
        relative_root = os.path.relpath(root.resolve(), repo_store)
        entry = field_string(1, name) + field_string(2, relative_root)
        index += bytes([0x0A]) + varint(len(entry)) + entry

    return bytes(index)


def parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Rebuild .jj/repo/workspace_store/index from workspace checkout paths.",
    )
    parser.add_argument(
        "--repo",
        type=Path,
        default=Path.cwd(),
        help="default workspace root containing .jj/repo (default: current directory)",
    )
    parser.add_argument(
        "workspace",
        nargs="+",
        type=parse_workspace,
        metavar="NAME=PATH",
        help=(
            "workspace name and checkout path, for example "
            "default=/code/repo feature=/code/repo.feature"
        ),
    )
    return parser


def main() -> int:
    args = parser().parse_args()

    repo_root = args.repo.resolve()
    repo_store = repo_root / ".jj" / "repo"
    if not repo_store.is_dir():
        print(f"error: '{repo_root}' does not contain .jj/repo", file=sys.stderr)
        return 1

    index_path = repo_store / "workspace_store" / "index"
    index_path.parent.mkdir(parents=True, exist_ok=True)

    backup_path = backup(index_path)
    index_path.write_bytes(build_index(repo_store, args.workspace))

    if backup_path is None:
        print(f"wrote {index_path}")
    else:
        print(f"backed up {index_path} to {backup_path}")
        print(f"wrote {index_path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
