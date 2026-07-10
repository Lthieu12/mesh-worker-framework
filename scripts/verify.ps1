# ============================================================================
# Verify MMF on Windows and Linux.
# ============================================================================

$ErrorActionPreference = "Stop"

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
        Invoke-Expression $Command
    }
    else {
        wsl bash -lc "cd '$project_path_linux' && $Command"
    }

    if ($LASTEXITCODE -ne 0) {
        throw "[$Platform] $Description"
    }

    Write-Host "[ OK ]"
}

try {

    # Get the Linux project path.
    $project_path_windows = (Get-Location).Path
    $project_path_linux = (wsl wslpath "$project_path_windows").Trim()

    Write-Host ""
    Write-Host "========================================"
    Write-Host "        MMF Verification"
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
    Write-Host "MMF verification completed successfully."
    Write-Host "========================================"

    exit 0
}
catch {

    Write-Host ""
    Write-Host "[FAIL] $($_.Exception.Message)"
    Write-Host ""
    Write-Host "========================================"
    Write-Host "MMF verification failed."
    Write-Host "========================================"

    exit 1
}
finally {

    Write-Host ""

}