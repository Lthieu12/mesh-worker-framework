# ============================================================================
# Commit and push all repository changes.
# ============================================================================
#   .\scripts\git_push.ps1 "commit info"

param(
    [Parameter(Mandatory = $true)]
    [string]$CommitMessage
)

$ErrorActionPreference = "Stop"

try {

    # Stage all changes.
    git add .

    if ($LASTEXITCODE -ne 0) {
        throw "git add ."
    }

    # Create the commit.
    git commit -m "$CommitMessage"

    if ($LASTEXITCODE -ne 0) {
        throw "git commit"
    }

    # Push to the remote repository.
    git push

    if ($LASTEXITCODE -ne 0) {
        throw "git push"
    }

    Write-Host ""
    Write-Host "========================================"
    Write-Host "Git push completed successfully."
    Write-Host "========================================"

    exit 0
}
catch {

    Write-Host ""
    Write-Host "[FAIL] $($_.Exception.Message)"
    Write-Host ""
    Write-Host "========================================"
    Write-Host "Git push failed."
    Write-Host "========================================"

    exit 1
}
finally {

    Write-Host ""

}