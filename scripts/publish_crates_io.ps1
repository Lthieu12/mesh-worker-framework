# Validate and publish the Cargo package to crates.io.
# Run this script from anywhere inside the repository:
#   .\scripts\publish_crates_io.ps1

[CmdletBinding()]
param()

Set-StrictMode -Version Latest

# Native command errors are checked explicitly through LASTEXITCODE.
# Keep stderr output from Git and Cargo visible without treating it as a PowerShell exception.
$ErrorActionPreference = 'Continue'
$PSNativeCommandUseErrorActionPreference = $false

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [switch]$Quiet
    )

    $output = & $Command @Arguments 2>&1
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0) {
        $details = ($output | Out-String).Trim()
        throw "$Command command failed (exit code ${exitCode}): $Command $($Arguments -join ' ')`n$details"
    }

    if (-not $Quiet) {
        if ($output) {
            $output | ForEach-Object { Write-Host $_ }
        }
        return
    }

    return [string]::Join([Environment]::NewLine, [string[]]@($output))
}

try {
    # Resolve the repository root, regardless of the current working directory.
    $repoRoot = (Invoke-NativeCommand -Command 'git' -Arguments @('rev-parse', '--show-toplevel') -Quiet).Trim()
    if ([string]::IsNullOrWhiteSpace($repoRoot)) {
        throw 'Unable to determine the Git repository root.'
    }
    Set-Location -LiteralPath $repoRoot

    # Read the package name and version from Cargo metadata.
    $manifestPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'Cargo.toml'))
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Cargo.toml was not found at the repository root: $manifestPath"
    }

    $metadataJson = Invoke-NativeCommand -Command 'cargo' -Arguments @('metadata', '--no-deps', '--format-version', '1', '--locked') -Quiet
    $metadata = $metadataJson | ConvertFrom-Json
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

    # Do not publish while local changes remain uncommitted.
    $status = (Invoke-NativeCommand -Command 'git' -Arguments @('status', '--porcelain') -Quiet).Trim()
    if (-not [string]::IsNullOrWhiteSpace($status)) {
        throw "The working tree is not clean. Commit or discard these changes before publishing:`n$status"
    }

    # Require the release tag for the Cargo package version to exist locally or on origin.
    # The tag is not required to point to the current HEAD; later commits are allowed.
    $tag = "v$version"
    $localTag = (Invoke-NativeCommand -Command 'git' -Arguments @('tag', '--list', $tag) -Quiet).Trim()
    $tagLocation = $null

    if ($localTag -eq $tag) {
        $tagLocation = 'local repository'
    }
    else {
        $originUrl = (Invoke-NativeCommand -Command 'git' -Arguments @('remote', 'get-url', 'origin') -Quiet).Trim()
        if ([string]::IsNullOrWhiteSpace($originUrl)) {
            throw "Release tag '$tag' was not found locally and no origin remote is configured."
        }

        $remoteTag = (Invoke-NativeCommand -Command 'git' -Arguments @('ls-remote', '--tags', 'origin', "refs/tags/$tag") -Quiet).Trim()
        if ([string]::IsNullOrWhiteSpace($remoteTag)) {
            throw "Release tag '$tag' was not found in the local repository or on origin."
        }

        $tagLocation = 'origin'
    }

    $head = (Invoke-NativeCommand -Command 'git' -Arguments @('log', '-1', '--oneline') -Quiet).Trim()

    Write-Host ''
    Write-Host 'Crates.io publication' -ForegroundColor Cyan
    Write-Host "  Repository : $repoRoot"
    Write-Host "  Package    : $packageName"
    Write-Host "  Version    : $version"
    Write-Host "  Git tag    : $tag ($tagLocation)"
    Write-Host "  Commit     : $head"
    Write-Host ''

    # Validate the package before asking permission to publish.
    Invoke-NativeCommand -Command 'cargo' -Arguments @('publish', '--dry-run', '--locked', '--package', $packageName)

    Write-Host ''
    Write-Host "The dry run completed successfully. Package '$packageName' version '$version' is ready to publish." -ForegroundColor Green
    $confirmation = Read-Host "Publish '$packageName' $version to crates.io? [y/N]"
    if ($confirmation -notmatch '^(?i:y|yes)$') {
        Write-Host 'Cancelled. The package was not published.' -ForegroundColor Yellow
        exit 0
    }

    # Publish only after explicit confirmation.
    Invoke-NativeCommand -Command 'cargo' -Arguments @('publish', '--locked', '--package', $packageName)

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
