"""Frozen pre-migration fixtures for historical renderer comparisons."""
from pathlib import Path
import subprocess

# The shared-language checkpoint still had the original canonical fixtures.
# Historical binaries cannot parse the replacement grammar. Compare equivalent
# authored fixtures in each binary's own grammar, rather than retaining a
# legacy parser in the current compiler.
LEGACY_FIXTURE_REVISION = "3d4eda6"


def legacy_source(root: Path, path: Path) -> str:
    relative = path.relative_to(root).as_posix()
    return subprocess.check_output(
        ["git", "show", f"{LEGACY_FIXTURE_REVISION}:{relative}"],
        cwd=root,
        text=True,
    )
