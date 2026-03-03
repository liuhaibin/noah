[CmdletBinding()]
param(
  [switch]$BuildOnly,
  [switch]$Upload,
  [string]$Tag,
  [switch]$SkipInstall
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Set-Location -Path $PSScriptRoot

$argsList = @()
if ($BuildOnly) { $argsList += '--build' }
if ($Upload) { $argsList += '--upload' }
if ($Tag) { $argsList += @('--tag', $Tag) }
if ($SkipInstall) { $argsList += '--skip-install' }

if (-not $BuildOnly -and -not $Upload) {
  $argsList += '--build'
}

node .\scripts\release.mjs @argsList
