# ============================================================================
# Create a release commit and annotated Git tag from the Cargo package version.
# ============================================================================
#   .\scripts\git_tag.ps1

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Handle native command failures through their exit codes instead of PowerShell's
# interpretation of stderr output (Git may emit harmless warnings on stderr).
$PSNativeCommandUseErrorActionPreference = $false

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [switch]$Quiet
    )

    # Git can write harmless warnings (for example, line-ending warnings) to stderr.
    # Temporarily allow that output and use Git's exit code to determine success.
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

try {
    # Resolve the repository root, regardless of the current working directory.
    $repoRoot = (Invoke-Git -Arguments @('rev-parse', '--show-toplevel') -Quiet).Trim()
    if ([string]::IsNullOrWhiteSpace($repoRoot)) {
        throw 'Unable to determine the Git repository root.'
    }
    Set-Location -LiteralPath $repoRoot

    # Read the version of the package whose manifest is at the repository root.
    $manifestPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'Cargo.toml'))
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Cargo.toml was not found at the repository root: $manifestPath"
    }

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $metadataOutput = & cargo metadata --no-deps --format-version 1
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

    $version = [string]$rootPackages[0].version
    if ($version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') {
        throw "The Cargo package version '$version' is not a valid semantic version."
    }

    $tag = "v$version"
    $commitMessage = "Release $tag"
    $tagMessage = "Release $tag"

    # Ensure this is a named branch and that the origin remote is configured.
    $branch = (Invoke-Git -Arguments @('branch', '--show-current') -Quiet).Trim()
    if ([string]::IsNullOrWhiteSpace($branch)) {
        throw 'The repository is in a detached HEAD state. Switch to a branch before tagging a release.'
    }

    [void](Invoke-Git -Arguments @('remote', 'get-url', 'origin') -Quiet)

    # Refuse to overwrite an existing local or remote release tag.
    $localTag = (Invoke-Git -Arguments @('tag', '--list', $tag) -Quiet).Trim()
    if ($localTag -eq $tag) {
        throw "The local tag '$tag' already exists. No changes were made."
    }

    $remoteTag = (Invoke-Git -Arguments @('ls-remote', '--tags', 'origin', "refs/tags/$tag") -Quiet).Trim()
    if (-not [string]::IsNullOrWhiteSpace($remoteTag)) {
        throw "The tag '$tag' already exists on origin. No changes were made."
    }

    # Show the release target and current changes before asking for confirmation.
    $status = (Invoke-Git -Arguments @('status', '--short') -Quiet).Trim()
    $head = (Invoke-Git -Arguments @('log', '-1', '--oneline') -Quiet).Trim()

    Write-Host ''
    Write-Host 'Release summary' -ForegroundColor Cyan
    Write-Host "  Repository : $repoRoot"
    Write-Host "  Branch     : $branch"
    Write-Host "  Version    : $version"
    Write-Host "  Tag        : $tag"
    Write-Host "  Commit     : $commitMessage"
    Write-Host "  Current HEAD: $head"
    Write-Host ''

    if ([string]::IsNullOrWhiteSpace($status)) {
        Write-Host 'Working tree is clean. The current HEAD will be tagged.' -ForegroundColor Yellow
    }
    else {
        Write-Host 'The following changes will be staged and committed:' -ForegroundColor Yellow
        Write-Host $status
    }

    Write-Host ''
    $confirmation = Read-Host "Proceed with commit (if needed), tag '$tag', and push to origin? [y/N]"
    if ($confirmation -notmatch '^(?i:y|yes)$') {
        Write-Host 'Cancelled. No changes were made.' -ForegroundColor Yellow
        exit 0
    }

    # Check whitespace errors, then commit all tracked and untracked changes.
    [void](Invoke-Git -Arguments @('diff', '--check') -Quiet)
    [void](Invoke-Git -Arguments @('diff', '--cached', '--check') -Quiet)

    if (-not [string]::IsNullOrWhiteSpace($status)) {
        [void](Invoke-Git -Arguments @('add', '--all') -Quiet)
        [void](Invoke-Git -Arguments @('diff', '--cached', '--check') -Quiet)

        $stagedFiles = (Invoke-Git -Arguments @('diff', '--cached', '--name-only') -Quiet).Trim()
        if (-not [string]::IsNullOrWhiteSpace($stagedFiles)) {
            [void](Invoke-Git -Arguments @('commit', '-m', $commitMessage))
        }
    }

    # Do not tag if changes remain after the release commit.
    $remainingChanges = (Invoke-Git -Arguments @('status', '--porcelain') -Quiet).Trim()
    if (-not [string]::IsNullOrWhiteSpace($remainingChanges)) {
        throw "The working tree is not clean after the commit. Resolve these changes before tagging:`n$remainingChanges"
    }

    # Create an annotated tag, then push the branch and the tag explicitly.
    [void](Invoke-Git -Arguments @('tag', '-a', $tag, '-m', $tagMessage))
    Write-Host "Created annotated tag $tag." -ForegroundColor Green

    [void](Invoke-Git -Arguments @('push', 'origin', $branch))
    [void](Invoke-Git -Arguments @('push', 'origin', $tag))

    Write-Host ''
    Write-Host "Release $tag committed, tagged, and pushed successfully." -ForegroundColor Green
}
catch {
    Write-Host ''
    Write-Host "ERROR: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host 'No automatic rollback was attempted. Check the Git status and local tags before retrying.' -ForegroundColor Yellow
    exit 1
}
