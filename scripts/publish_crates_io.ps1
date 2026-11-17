# Validate and publish the Cargo package to crates.io.
# Run this script from anywhere inside the repository:
#   .\scripts\publish_crates_io.ps1

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Handle native command failures through their exit codes instead of PowerShell's
# interpretation of stderr output (Cargo may emit status messages on stderr).
$PSNativeCommandUseErrorActionPreference = $false

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [switch]$Quiet
    )

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = & git @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    if ($exitCode -ne 0) {
        $details = ($output | Out-String).Trim()
        throw "Git command failed (exit code ${exitCode}): git $($Arguments -join ' ')`n$details"
    }

    if (-not $Quiet -and $output) {
        $output | ForEach-Object { Write-Host $_ }
    }

    return ($output -join [Environment]::NewLine)
}

function Invoke-Cargo {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    Write-Host "`n> cargo $($Arguments -join ' ')" -ForegroundColor DarkCyan

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = & cargo @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    if ($output) {
        $output | ForEach-Object { Write-Host $_ }
    }

    if ($exitCode -ne 0) {
        throw "Cargo command failed (exit code ${exitCode}): cargo $($Arguments -join ' ')"
    }
}

try {
    # Resolve the repository root, regardless of the current working directory.
    $repoRoot = (Invoke-Git -Arguments @('rev-parse', '--show-toplevel') -Quiet).Trim()
    if ([string]::IsNullOrWhiteSpace($repoRoot)) {
        throw 'Unable to determine the Git repository root.'
    }
    Set-Location -LiteralPath $repoRoot

    # Read the package name and version from Cargo metadata.
    $manifestPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'Cargo.toml'))
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Cargo.toml was not found at the repository root: $manifestPath"
    }

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $metadataOutput = & cargo metadata --no-deps --format-version 1 --locked
        $cargoExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($cargoExitCode -ne 0) {
        throw "cargo metadata failed (exit code ${cargoExitCode})."
    }

    $metadata = ($metadataOutput -join [Environment]::NewLine) | ConvertFrom-Json
    $rootPackages = @($metadata.packages | Where-Object {
        [System.IO.Path]::GetFullPath($_.manifest_path) -eq $manifestPath
    })

    if ($rootPackages.Count -ne 1) {
        throw 'Could not identify exactly one Cargo package from the root Cargo.toml.'
    }

    $packageName = [string]$rootPackages[0].name
    $version = [string]$rootPackages[0].version
    if ($version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') {
        throw "The Cargo package version '$version' is not a valid semantic version."
    }

    $tag = "v$version"

    # Ensure the release tag exists and points to the current commit.
    $headCommit = (Invoke-Git -Arguments @('rev-parse', 'HEAD') -Quiet).Trim()
    $tagCommit = (Invoke-Git -Arguments @('rev-parse', "$tag^{commit}") -Quiet).Trim()
    if ($tagCommit -ne $headCommit) {
        throw "The release tag '$tag' does not point to the current HEAD. Run git_tag.ps1 for this version before publishing."
    }

    # Do not publish from a working tree that differs from the tagged commit.
    $status = (Invoke-Git -Arguments @('status', '--porcelain') -Quiet).Trim()
    if (-not [string]::IsNullOrWhiteSpace($status)) {
        throw "The working tree is not clean. Commit or discard these changes before publishing:`n$status"
    }

    $head = (Invoke-Git -Arguments @('log', '-1', '--oneline') -Quiet).Trim()

    Write-Host ''
    Write-Host 'Crates.io publication' -ForegroundColor Cyan
    Write-Host "  Repository : $repoRoot"
    Write-Host "  Package    : $packageName"
    Write-Host "  Version    : $version"
    Write-Host "  Git tag    : $tag"
    Write-Host "  Commit     : $head"
    Write-Host ''

    # Validate the package before asking permission to publish it.
    Invoke-Cargo -Arguments @('publish', '--dry-run', '--locked', '--package', $packageName)

    Write-Host ''
    Write-Host "The dry run completed successfully. Package '$packageName' version '$version' is ready to publish." -ForegroundColor Green
    $confirmation = Read-Host "Publish '$packageName' $version to crates.io? [y/N]"
    if ($confirmation -notmatch '^(?i:y|yes)$') {
        Write-Host 'Cancelled. The package was not published.' -ForegroundColor Yellow
        exit 0
    }

    # Publish only after explicit confirmation.
    Invoke-Cargo -Arguments @('publish', '--locked', '--package', $packageName)

    Write-Host ''
    Write-Host "Cargo successfully published '$packageName' version '$version' to crates.io." -ForegroundColor Green
    Write-Host "Package page: https://crates.io/crates/$packageName" -ForegroundColor Cyan
}
catch {
    Write-Host ''
    Write-Host "ERROR: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host 'No automatic rollback was attempted. Check the package and crates.io status before retrying.' -ForegroundColor Yellow
    exit 1
}
