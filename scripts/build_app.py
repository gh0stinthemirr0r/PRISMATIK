#!/usr/bin/env python3
"""Install what PRISMATIK needs on this host, then build the desktop bundle.

Tauri's prerequisites are not the same on the three platforms we ship to, and
the differences are not cosmetic: Linux needs a WebKit development package
whose name varies by distribution, macOS needs the Xcode command line tools,
and Windows needs the MSVC toolchain and WebView2. This script resolves that
per host so ``pnpm tauri build`` has what it needs.

Two rules shape it:

**Nothing privileged or networked runs without consent.** Installing system
packages and piping rustup's installer from the network are both things a
build script should ask about rather than assume. Every such command is
printed in full and confirmed, unless ``--yes`` is passed — which is what CI
should use, having made that decision deliberately.

**It does not pretend to install what it cannot.** Visual Studio Build Tools
and the Xcode command line tools are interactive installers; this script
detects their absence, says exactly what to run, and stops. Reporting success
and then failing three minutes into a link step is the worse outcome.

Exit codes: 0 success, 1 a prerequisite is missing or a step failed,
2 the platform is unsupported.
"""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Pinned in package.json; CI uses the same pair.
REQUIRED_NODE_MAJOR = 22
PNPM_VERSION = "10.13.1"

RUSTUP_URL = "https://sh.rustup.rs"


class Abort(Exception):
    """A prerequisite is missing and the script cannot resolve it itself."""


# --------------------------------------------------------------------------
# Output
# --------------------------------------------------------------------------

# Colour only when stdout is a terminal that is not explicitly opting out.
_COLOUR = sys.stdout.isatty() and os.environ.get("NO_COLOR") is None


def _paint(code: str, text: str) -> str:
    return f"\033[{code}m{text}\033[0m" if _COLOUR else text


def step(message: str) -> None:
    print(_paint("1;36", f"\n==> {message}"))


def info(message: str) -> None:
    print(f"    {message}")


def ok(message: str) -> None:
    print(f"    {_paint('32', 'ok')} {message}")


def warn(message: str) -> None:
    print(f"    {_paint('33', 'warning')} {message}")


def fail(message: str) -> None:
    print(f"    {_paint('31', 'error')} {message}", file=sys.stderr)


# --------------------------------------------------------------------------
# Host detection
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class LinuxPackages:
    """One distribution family's install command and package list."""

    manager: str
    install: list[str]
    packages: list[str]
    # Some managers need a separate refresh before install resolves names.
    refresh: list[str] = field(default_factory=list)


# Package names differ per family for the same underlying libraries. The
# WebKit development package is the one that actually varies; the rest are
# build essentials that Tauri's linking step needs.
LINUX_FAMILIES: dict[str, LinuxPackages] = {
    "debian": LinuxPackages(
        manager="apt-get",
        refresh=["apt-get", "update"],
        install=["apt-get", "install", "-y"],
        packages=[
            "libwebkit2gtk-4.1-dev",
            "build-essential",
            "curl",
            "wget",
            "file",
            "libxdo-dev",
            "libssl-dev",
            "libayatana-appindicator3-dev",
            "librsvg2-dev",
            "patchelf",
        ],
    ),
    "fedora": LinuxPackages(
        manager="dnf",
        install=["dnf", "install", "-y"],
        packages=[
            "webkit2gtk4.1-devel",
            "openssl-devel",
            "curl",
            "wget",
            "file",
            "libappindicator-gtk3-devel",
            "librsvg2-devel",
            "patchelf",
            "gcc",
            "gcc-c++",
            "make",
        ],
    ),
    "arch": LinuxPackages(
        manager="pacman",
        install=["pacman", "-S", "--needed", "--noconfirm"],
        packages=[
            "webkit2gtk-4.1",
            "base-devel",
            "curl",
            "wget",
            "file",
            "openssl",
            "libappindicator-gtk3",
            "librsvg",
            "patchelf",
        ],
    ),
    "suse": LinuxPackages(
        manager="zypper",
        install=["zypper", "install", "-y"],
        packages=[
            "webkit2gtk3-soup2-devel",
            "libopenssl-devel",
            "curl",
            "wget",
            "file",
            "libappindicator3-1",
            "librsvg-devel",
            "patchelf",
            "gcc",
            "gcc-c++",
            "make",
        ],
    ),
}


def linux_family() -> str | None:
    """Resolve the distribution family from ``/etc/os-release``.

    ``ID_LIKE`` is consulted after ``ID`` so derivatives (Mint, Pop!_OS,
    Manjaro, Rocky) resolve to the family whose package names they share.
    """
    release = Path("/etc/os-release")
    if not release.exists():
        return None

    fields: dict[str, str] = {}
    for line in release.read_text(encoding="utf-8", errors="replace").splitlines():
        key, sep, value = line.partition("=")
        if sep:
            fields[key.strip()] = value.strip().strip('"')

    candidates = [fields.get("ID", "")]
    candidates += fields.get("ID_LIKE", "").split()

    aliases = {
        "debian": "debian",
        "ubuntu": "debian",
        "linuxmint": "debian",
        "pop": "debian",
        "raspbian": "debian",
        "fedora": "fedora",
        "rhel": "fedora",
        "centos": "fedora",
        "rocky": "fedora",
        "almalinux": "fedora",
        "arch": "arch",
        "archlinux": "arch",
        "manjaro": "arch",
        "endeavouros": "arch",
        "opensuse": "suse",
        "opensuse-leap": "suse",
        "opensuse-tumbleweed": "suse",
        "sles": "suse",
        "suse": "suse",
    }
    for candidate in candidates:
        if candidate in aliases:
            return aliases[candidate]
    return None


# --------------------------------------------------------------------------
# Command execution
# --------------------------------------------------------------------------


@dataclass
class Runner:
    """Runs commands, asking before anything privileged or networked."""

    assume_yes: bool
    dry_run: bool

    def confirm(self, prompt: str) -> bool:
        if self.assume_yes:
            return True
        if not sys.stdin.isatty():
            # Non-interactive with no --yes: refusing beats guessing, because
            # the caller never saw the prompt and cannot have consented.
            fail(f"{prompt} — needs --yes when stdin is not a terminal")
            return False
        try:
            answer = input(f"    {prompt} [y/N] ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print()
            return False
        return answer in ("y", "yes")

    def run(self, command: list[str], *, cwd: Path | None = None) -> int:
        printable = " ".join(command)
        if self.dry_run:
            info(f"would run: {printable}")
            return 0
        info(f"$ {printable}")
        return subprocess.call(command, cwd=str(cwd) if cwd else None)

    def run_checked(self, command: list[str], *, cwd: Path | None = None) -> None:
        code = self.run(command, cwd=cwd)
        if code != 0:
            raise Abort(f"`{' '.join(command)}` exited {code}")

    def run_privileged(self, command: list[str], *, why: str) -> None:
        """Run a command as root, asking first and showing exactly what runs."""
        is_root = getattr(os, "geteuid", lambda: 1)() == 0
        full = command if is_root else ["sudo", *command]
        info(why)
        info(f"    {' '.join(full)}")
        if not self.confirm("Run this with elevated privileges?"):
            raise Abort("declined; install the packages listed above by hand")
        self.run_checked(full)


def which(name: str) -> str | None:
    return shutil.which(name)


def output_of(command: list[str]) -> str | None:
    try:
        result = subprocess.run(
            command, capture_output=True, text=True, timeout=60, check=False
        )
    except (OSError, subprocess.SubprocessError):
        return None
    return result.stdout.strip() if result.returncode == 0 else None


# --------------------------------------------------------------------------
# Per-platform system dependencies
# --------------------------------------------------------------------------


def install_linux_deps(runner: Runner) -> None:
    step("Linux system libraries")
    family = linux_family()
    if family is None:
        raise Abort(
            "could not identify this distribution from /etc/os-release. Install "
            "Tauri's Linux prerequisites manually: "
            "https://tauri.app/start/prerequisites/"
        )

    spec = LINUX_FAMILIES[family]
    info(f"distribution family: {family} ({spec.manager})")

    if which(spec.manager) is None:
        raise Abort(f"{spec.manager} is not on PATH; cannot install system packages")

    if spec.refresh:
        runner.run_privileged(spec.refresh, why="Refresh the package index:")

    runner.run_privileged(
        [*spec.install, *spec.packages],
        why=f"Install {len(spec.packages)} Tauri build dependencies:",
    )
    ok("system libraries installed")


def install_macos_deps(runner: Runner) -> None:
    step("macOS system tools")

    # The compiler and linker come from the command line tools. There is no
    # unattended install: `xcode-select --install` opens a GUI dialog, so the
    # honest move is to detect and instruct rather than to appear to handle it.
    if output_of(["xcode-select", "-p"]) is None:
        raise Abort(
            "Xcode command line tools are not installed. Run:\n"
            "        xcode-select --install\n"
            "    then re-run this script once the installer finishes."
        )
    ok("Xcode command line tools present")

    # Tauri on macOS needs nothing further; Homebrew is not a requirement.
    if which("brew") is None:
        info("Homebrew not found — not required for this build")


def install_windows_deps(runner: Runner) -> None:
    step("Windows system tools")

    # The MSVC toolchain is what actually links the binary. Its installer is
    # multi-gigabyte and interactive, so detect and instruct.
    has_msvc = which("cl") is not None or any(
        Path(root).exists()
        for root in (
            r"C:\Program Files\Microsoft Visual Studio",
            r"C:\Program Files (x86)\Microsoft Visual Studio",
            r"C:\Program Files (x86)\Microsoft Visual C++ Build Tools",
        )
    )
    if not has_msvc:
        raise Abort(
            "the MSVC build tools were not found. Install the "
            '"Desktop development with C++" workload:\n'
            "        winget install --id Microsoft.VisualStudio.2022.BuildTools "
            "--override "
            '"--quiet --add Microsoft.VisualStudio.Workload.VCTools '
            '--includeRecommended"\n'
            "    then re-run this script."
        )
    ok("MSVC build tools present")

    # WebView2 ships with Windows 11 and current Windows 10. Its absence is
    # only a runtime problem, not a build one, so this warns rather than stops.
    webview_keys = [
        Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"))
        / "Microsoft"
        / "EdgeWebView",
    ]
    if not any(path.exists() for path in webview_keys):
        warn(
            "WebView2 runtime not detected. The build will still succeed, but the "
            "app needs it at runtime: winget install --id Microsoft.EdgeWebView2Runtime"
        )
    else:
        ok("WebView2 runtime present")


# --------------------------------------------------------------------------
# Toolchains
# --------------------------------------------------------------------------


def ensure_rust(runner: Runner, host: str) -> None:
    step("Rust toolchain")

    if which("cargo") is not None:
        version = output_of(["cargo", "--version"]) or "unknown version"
        ok(version)
        # rust-toolchain.toml pins the channel, so rustup selects it on its own
        # the first time cargo runs in the repository.
        if which("rustup") is not None:
            runner.run(["rustup", "show", "active-toolchain"], cwd=REPO_ROOT)
        return

    if host == "windows":
        raise Abort(
            "Rust is not installed. Install it with:\n"
            "        winget install --id Rustlang.Rustup\n"
            "    then re-run this script."
        )

    # Piping a remote script into a shell is exactly the kind of thing that
    # should be a decision rather than a default, so it is named and confirmed.
    info(f"Rust is not installed. The official installer is {RUSTUP_URL}.")
    info("This downloads and executes an installation script from the network.")
    if not runner.confirm(f"Download and run the rustup installer from {RUSTUP_URL}?"):
        raise Abort(
            "declined. Install Rust yourself from https://rustup.rs and re-run."
        )

    if runner.dry_run:
        info(f"would run: curl --proto '=https' --tlsv1.2 -sSf {RUSTUP_URL} | sh -s -- -y")
        return

    curl = subprocess.Popen(
        ["curl", "--proto", "=https", "--tlsv1.2", "-sSf", RUSTUP_URL],
        stdout=subprocess.PIPE,
    )
    shell = subprocess.Popen(["sh", "-s", "--", "-y"], stdin=curl.stdout)
    if curl.stdout is not None:
        curl.stdout.close()
    shell.communicate()
    if shell.returncode != 0:
        raise Abort("the rustup installer failed")

    cargo_bin = Path.home() / ".cargo" / "bin"
    os.environ["PATH"] = f"{cargo_bin}{os.pathsep}{os.environ.get('PATH', '')}"
    if which("cargo") is None:
        raise Abort(
            f"Rust installed but cargo is not on PATH. Add {cargo_bin} to PATH "
            "and re-run."
        )
    ok("Rust installed")


def ensure_node(runner: Runner) -> None:
    step("Node.js and pnpm")

    if which("node") is None:
        raise Abort(
            f"Node.js {REQUIRED_NODE_MAJOR} is not installed. Install it from "
            "https://nodejs.org or through your version manager (nvm, fnm, "
            "volta), then re-run."
        )

    version = output_of(["node", "--version"]) or ""
    ok(f"node {version or 'unknown'}")
    try:
        major = int(version.lstrip("v").split(".")[0])
    except (ValueError, IndexError):
        warn("could not parse the Node version; continuing")
    else:
        if major < REQUIRED_NODE_MAJOR:
            raise Abort(
                f"Node {major} is too old; this project builds against "
                f"{REQUIRED_NODE_MAJOR}. Upgrade and re-run."
            )

    if which("pnpm") is not None:
        ok(output_of(["pnpm", "--version"]) or "pnpm present")
        return

    # corepack ships with Node and pins the exact version package.json names,
    # which is why it is preferred over a global npm install.
    if which("corepack") is not None:
        info(f"pnpm is missing; enabling it through corepack at {PNPM_VERSION}")
        if runner.run(["corepack", "enable"]) == 0 and (
            runner.run(["corepack", "prepare", f"pnpm@{PNPM_VERSION}", "--activate"])
            == 0
        ):
            ok("pnpm activated")
            return
        warn("corepack could not activate pnpm; falling back to npm")

    if not runner.confirm(f"Install pnpm@{PNPM_VERSION} globally with npm?"):
        raise Abort("declined; install pnpm yourself and re-run")
    runner.run_checked(["npm", "install", "-g", f"pnpm@{PNPM_VERSION}"])
    ok("pnpm installed")


# --------------------------------------------------------------------------
# Build
# --------------------------------------------------------------------------


def build(runner: Runner, *, debug: bool) -> None:
    step("Installing workspace packages")
    runner.run_checked(["pnpm", "install", "--frozen-lockfile"], cwd=REPO_ROOT)

    step("Building the desktop application")
    command = ["pnpm", "tauri", "build"]
    if debug:
        # A debug bundle links far faster, which is what you want when the
        # question is "does this host build at all" rather than "ship it".
        command.append("--debug")
    runner.run_checked(command, cwd=REPO_ROOT)

    profile = "debug" if debug else "release"
    bundle = REPO_ROOT / "apps/desktop/src-tauri/target" / profile / "bundle"
    step("Done")
    if bundle.exists():
        ok(f"bundles written to {bundle}")
        for path in sorted(bundle.rglob("*")):
            if path.is_file() and path.suffix in {
                ".deb",
                ".rpm",
                ".AppImage",
                ".dmg",
                ".app",
                ".msi",
                ".exe",
            }:
                size_mb = path.stat().st_size / (1024 * 1024)
                info(f"{path.relative_to(bundle)} ({size_mb:.1f} MB)")
    else:
        # Not an error: `tauri build` succeeds without bundling when the
        # config disables it, and claiming a missing artifact would be worse.
        warn(f"no bundle directory at {bundle}; the binary is under target/{profile}")


# --------------------------------------------------------------------------
# Entry point
# --------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Install PRISMATIK's build dependencies for this platform, "
        "then compile the desktop application.",
    )
    parser.add_argument(
        "-y",
        "--yes",
        action="store_true",
        help="assume yes for privileged and networked steps (what CI should use)",
    )
    parser.add_argument(
        "--skip-deps",
        action="store_true",
        help="go straight to the build, assuming prerequisites are already met",
    )
    parser.add_argument(
        "--deps-only",
        action="store_true",
        help="install prerequisites and stop without building",
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="build a debug bundle (much faster to link)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print every command without running it",
    )
    args = parser.parse_args()

    system = platform.system()
    host = {"Linux": "linux", "Darwin": "macos", "Windows": "windows"}.get(system)
    if host is None:
        fail(f"unsupported platform: {system}")
        return 2

    print(_paint("1", "PRISMATIK build"))
    info(f"host      {host} ({platform.machine()})")
    info(f"repo      {REPO_ROOT}")
    if args.dry_run:
        info("dry run   no command will actually execute")

    runner = Runner(assume_yes=args.yes, dry_run=args.dry_run)

    try:
        if not args.skip_deps:
            if host == "linux":
                install_linux_deps(runner)
            elif host == "macos":
                install_macos_deps(runner)
            else:
                install_windows_deps(runner)

            ensure_rust(runner, host)
            ensure_node(runner)
        else:
            step("Skipping dependency installation (--skip-deps)")

        if args.deps_only:
            step("Prerequisites ready (--deps-only); not building")
            return 0

        build(runner, debug=args.debug)
    except Abort as error:
        fail(str(error))
        return 1
    except KeyboardInterrupt:
        print()
        fail("interrupted")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
