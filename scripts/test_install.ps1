$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'install.ps1')

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) {
        throw "$Message expected=$Expected actual=$Actual"
    }
}

function Assert-True {
    param([bool]$Value, [string]$Message)
    if (-not $Value) {
        throw $Message
    }
}

function Assert-False {
    param([bool]$Value, [string]$Message)
    if ($Value) {
        throw $Message
    }
}

Assert-Equal (Get-ReleaseTarget) 'x86_64-pc-windows-msvc' 'release target mismatch'
Assert-Equal (Get-PackageVersion -RepoRoot $RepoRoot) '0.2.0' 'package version mismatch'

$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("harness-install-" + [guid]::NewGuid())
$CodexHome = Join-Path $TempRoot 'codex-home'
try {
    Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome $CodexHome -BinarySource None
    $SkillRoot = Join-Path $CodexHome 'skills/cached-subagent-harness'
    Assert-True (Test-Path -LiteralPath (Join-Path $SkillRoot 'SKILL.md') -PathType Leaf) 'Skill was not installed'
    Assert-False (Test-Path -LiteralPath (Join-Path $SkillRoot 'scripts/bin/harnessctl.exe')) 'None source installed a Windows runtime'
    Assert-False (Test-Path -LiteralPath (Join-Path $SkillRoot 'scripts/bin/harnessctl')) 'None source copied a stale Unix runtime'
}
finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output 'PowerShell installer smoke passed'
