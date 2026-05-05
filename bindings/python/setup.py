from __future__ import annotations

import platform
import shutil
import subprocess
from pathlib import Path

from setuptools import setup
from setuptools.command.build_py import build_py as _build_py

try:
    from wheel.bdist_wheel import bdist_wheel as _bdist_wheel
except Exception:  # pragma: no cover - wheel is declared as a build dependency.
    _bdist_wheel = None


PACKAGE_DIR = Path(__file__).resolve().parent
REPO_ROOT = PACKAGE_DIR.parents[1]


def _library_name() -> str:
    system = platform.system()
    if system == "Windows":
        return "metricchrono_ffi.dll"
    if system == "Darwin":
        return "libmetricchrono_ffi.dylib"
    return "libmetricchrono_ffi.so"


class build_py(_build_py):
    def run(self) -> None:
        subprocess.run(
            ["cargo", "build", "-p", "metricchrono-ffi", "--release"],
            cwd=REPO_ROOT,
            check=True,
        )
        super().run()
        self._copy_native_library()

    def _copy_native_library(self) -> None:
        library = _library_name()
        source = REPO_ROOT / "target" / "release" / library
        if not source.exists():
            msg = f"expected native library at {source}"
            raise FileNotFoundError(msg)
        destination = Path(self.build_lib) / "metricchrono" / library
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)


cmdclass = {"build_py": build_py}

if _bdist_wheel is not None:

    class bdist_wheel(_bdist_wheel):
        def finalize_options(self) -> None:
            super().finalize_options()
            self.root_is_pure = False

    cmdclass["bdist_wheel"] = bdist_wheel


setup(cmdclass=cmdclass)
