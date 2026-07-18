# ============================================================================
# Verify MWF on Windows and Linux.
# ============================================================================

$ErrorActionPreference = "Stop"

Remove-Item target-windows -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item target-linux -Recurse -Force -ErrorAction SilentlyContinue

# Execute a verification step.
function InvokeVerificationStep {
    param(
        [string]$Platform,
        [string]$Description,
        [string]$Command
    )

    Write-Host ""
    Write-Host "[RUN ] $Description"

    if ($Platform -eq "Windows") {
        $env:CARGO_TARGET_DIR = $windows_target_dir
        Invoke-Expression $Command
    }
    else {
        wsl bash -lc "cd '$project_path_linux' && export CARGO_TARGET_DIR='$linux_target_dir' && $Command"
    }

    if ($LASTEXITCODE -ne 0) {
        throw "[$Platform] $Description"
    }

    Write-Host "[ OK ]"
}

# Save the current Cargo target directory, if any.
$had_cargo_target_dir = Test-Path Env:CARGO_TARGET_DIR
$previous_cargo_target_dir = $env:CARGO_TARGET_DIR

try {

    # Get the Linux project path.
    $project_path_windows = (Get-Location).Path
    $project_path_linux = (wsl wslpath "$project_path_windows").Trim()

    # Define platform-specific target directories.
    $windows_target_dir = "target-windows"
    $linux_target_dir = "target-linux"

    Write-Host ""
    Write-Host "========================================"
    Write-Host "        MWF Verification"
    Write-Host "========================================"

    # ========================================================================
    # Windows verification.
    # ========================================================================

    Write-Host ""
    Write-Host "----------------Windows-----------------"

    InvokeVerificationStep "Windows" "cargo fmt --check" "cargo fmt --check"
    InvokeVerificationStep "Windows" "cargo clippy" "cargo clippy --workspace --lib --examples --all-features -- -D warnings"
    InvokeVerificationStep "Windows" "cargo test" "cargo test --workspace --all-targets --all-features"
    InvokeVerificationStep "Windows" "cargo test --doc" "cargo test --workspace --doc"
    InvokeVerificationStep "Windows" "cargo build --examples" "cargo build --workspace --examples --all-features"
    InvokeVerificationStep "Windows" "cargo doc" "cargo doc --workspace --no-deps"

    # ========================================================================
    # Linux verification.
    # ========================================================================

    Write-Host ""
    Write-Host "-----------------Linux------------------"

    InvokeVerificationStep "Linux" "cargo fmt --check" "cargo fmt --check"
    InvokeVerificationStep "Linux" "cargo clippy" "cargo clippy --workspace --lib --examples --all-features -- -D warnings"
    InvokeVerificationStep "Linux" "cargo test" "cargo test --workspace --all-targets --all-features"
    InvokeVerificationStep "Linux" "cargo test --doc" "cargo test --workspace --doc"
    InvokeVerificationStep "Linux" "cargo build --examples" "cargo build --workspace --examples --all-features"
    InvokeVerificationStep "Linux" "cargo doc" "cargo doc --workspace --no-deps"

    Write-Host ""
    Write-Host "========================================"
    Write-Host "MWF verification completed successfully."
    Write-Host "========================================"

    exit 0
}
catch {

    Write-Host ""
    Write-Host "[FAIL] $($_.Exception.Message)"
    Write-Host ""
    Write-Host "========================================"
    Write-Host "MWF verification failed."
    Write-Host "========================================"

    exit 1
}
finally {

    # Restore the previous Cargo target directory.
    if ($had_cargo_target_dir) {
        $env:CARGO_TARGET_DIR = $previous_cargo_target_dir
    }
    else {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    }

    Write-Host ""

}