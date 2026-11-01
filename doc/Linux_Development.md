# Linux Verification Environment (WSL2)

## Objective

The goal of this setup is to provide a **native Linux development and verification environment** alongside the existing Windows environment, without using a dual boot or a traditional virtual machine.

This environment will allow MWF to:

* Compile with the native Linux Rust toolchain.
* Run unit tests under Linux.
* Run documentation tests (`cargo test --doc`).
* Compile all examples.
* Verify that MWF remains fully cross-platform (Windows + Linux).
* Execute a future validation script that will automatically run all Windows checks followed by the Linux checks.

WSL2 provides a lightweight Linux environment fully integrated into Windows while keeping the existing development workflow unchanged.

---

# Installation Procedure

## 1. Install WSL2

Open **PowerShell as Administrator** and execute:

```powershell
wsl --install
```

This installs:

* Windows Subsystem for Linux (WSL2)
* Virtual Machine Platform
* The WSL runtime

A Windows reboot is required before continuing.

---

## 2. Verify the Installation

After rebooting, verify that WSL is correctly installed:

```powershell
wsl --status
```

If no Linux distribution is installed yet, WSL will indicate that a distribution must be installed.

---

## 3. List Available Linux Distributions

Display all available distributions:

```powershell
wsl --list --online
```

Typical output:

```text
NAME
Ubuntu
Ubuntu-24.04
Ubuntu-22.04
...
```

---

## 4. Install Ubuntu 24.04 LTS

Install Ubuntu 24.04 LTS:

```powershell
wsl --install -d Ubuntu-24.04
```

During the first launch, Ubuntu asks for:

* A Linux username
* A Linux password

Example:

```text
Create a default Unix user account: laurent
New password:
Retype new password:
```

After the installation completes, a Linux shell is opened:

```text
laurent@PC-Thieu:~$
```

---

## 5. Verify the Linux Environment

Confirm that the system is running under WSL2:

```bash
uname -a
```

Example output:

```text
Linux PC-Thieu ... microsoft-standard-WSL2 ...
```

---

## 6. Update Ubuntu

Update the package database:

```bash
sudo apt update
```

Then upgrade installed packages:

```bash
sudo apt upgrade -y
```

---

## 7. Install Rust

Install the Linux Rust toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Choose the default installation.

Once finished, load Cargo into the current shell:

```bash
source "$HOME/.cargo/env"
```

---

## 8. Install the Native Build Tools

Many Rust crates require a native C toolchain.

Install it with:

```bash
sudo apt install -y build-essential pkg-config
```

This installs, among others:

* gcc
* g++
* make
* cc

---

## 9. Verify the Toolchain

Verify that everything is correctly installed:

```bash
cargo --version
rustc --version
cc --version
```

Example:

```text
cargo 1.97.0
rustc 1.97.0
cc (Ubuntu 13.3.0 ...)
```

---

# Working on the Windows Project

The project remains stored on the Windows filesystem.

Example:

```bash
cd "/mnt/c/Users/<WindowsUser>/Documents/Dev Rust/mesh-worker-framework"
```

No duplicate copy of the project is required.

Windows and Linux work on the exact same files.

---

# Current Result

The development machine now provides:

* Native Windows Rust toolchain
* Native Linux Rust toolchain (WSL2)
* Native Linux compilation
* Native Linux testing

At this point, MWF can be compiled and tested on both supported operating systems.

---

# Verification Script

The repository includes a PowerShell verification script:

`scripts/verify.ps1`

Run the script from the project root using PowerShell:

```powershell
.\scripts\verify.ps1

The script performs the complete verification pipeline on both Windows
and Linux (WSL2).

For each platform, it executes:
1. cargo fmt --check
2. cargo clippy with warnings treated as errors.
3. cargo test across all targets and features.
4. cargo test --doc
5. cargo build --examples
6. Execution of the getting_started examples and runtime benchmark.
7. cargo doc
Separate Cargo target directories are used for Windows and Linux
to avoid sharing incompatible build artifacts.

The script provides a single validation command to run before each
release.