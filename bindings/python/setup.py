from __future__ import annotations

import platform
import shutil
import subprocess
from pathlib import Path

from setuptools import setup
from setuptools.dist import Distribution
from setuptools.command.build_py import build_py as _build_py
from setuptools.command.sdist import sdist as _sdist

try:
    from setuptools.command.bdist_wheel import bdist_wheel as _bdist_wheel
except Exception:  # pragma: no cover - wheel is declared as a build dependency.
    try:
        from wheel.bdist_wheel import bdist_wheel as _bdist_wheel
    except Exception:
        _bdist_wheel = None


PACKAGE_DIR = Path(__file__).resolve().parent
RUST_SNAPSHOT = PACKAGE_DIR / "rust"


def _repo_root() -> Path:
    if (RUST_SNAPSHOT / "Cargo.toml").exists():
        return RUST_SNAPSHOT
    repo_root = PACKAGE_DIR.parents[1]
    if (repo_root / "Cargo.toml").exists():
        return repo_root
    msg = "could not find MetricChrono Rust workspace for native build"
    raise FileNotFoundError(msg)


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
            cwd=_repo_root(),
            check=True,
        )
        super().run()
        self._copy_native_library()

    def _copy_native_library(self) -> None:
        library = _library_name()
        source = _repo_root() / "target" / "release" / library
        if not source.exists():
            msg = f"expected native library at {source}"
            raise FileNotFoundError(msg)
        destination = Path(self.build_lib) / "metricchrono" / library
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)


class sdist(_sdist):
    def make_release_tree(self, base_dir: str, files: list[str]) -> None:
        super().make_release_tree(base_dir, files)
        self._copy_rust_workspace(Path(base_dir) / "rust")

    def _copy_rust_workspace(self, destination: Path) -> None:
        if destination.exists():
            shutil.rmtree(destination)
        repo_root = _repo_root()
        destination.mkdir(parents=True)
        shutil.copy2(repo_root / "Cargo.toml", destination / "Cargo.toml")
        lockfile = repo_root / "Cargo.lock"
        if lockfile.exists():
            shutil.copy2(lockfile, destination / "Cargo.lock")
        for crate in [
            "metricchrono-core",
            "metricchrono-ffi",
        ]:
            source = repo_root / "crates" / crate
            crate_destination = destination / "crates" / crate
            shutil.copytree(
                source,
                crate_destination,
                ignore=shutil.ignore_patterns("target", "*.dylib", "*.so", "*.dll"),
            )


cmdclass = {"build_py": build_py, "sdist": sdist}

if _bdist_wheel is not None:

    class bdist_wheel(_bdist_wheel):
        def finalize_options(self) -> None:
            super().finalize_options()
            self.root_is_pure = False

        def get_tag(self) -> tuple[str, str, str]:
            # The wheel bundles a prebuilt cdylib loaded via ctypes — it is
            # platform-specific but has no CPython ABI link, so a single wheel
            # works on every Python 3.x. Tag it py3-none-<platform>, not cpXY.
            _, _, plat = super().get_tag()
            return "py3", "none", plat

    cmdclass["bdist_wheel"] = bdist_wheel


class _BinaryDistribution(Distribution):
    # Force a platform (platlib) wheel so the bundled cdylib lands inside the
    # package dir, where auditwheel/delocate can find it and retag the wheel to
    # manylinux/macOS. There is no real extension module; the bdist_wheel
    # get_tag override keeps the wheel tag py3-none.
    def has_ext_modules(self) -> bool:
        return True


setup(cmdclass=cmdclass, distclass=_BinaryDistribution)
