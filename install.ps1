$ErrorActionPreference = 'Stop'

$repo = "Ashutosh0x/arc-cli"
$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$version = $release.tag_name
$asset = $release.assets | Where-Object { $_.name -match "windows-msvc" }

if (-not $asset) { throw "No Windows binary found in release $version" }

$tmp = Join-Path $env:TEMP "arc-cli-install"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

$zip = Join-Path $tmp $asset.name
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip
Expand-Archive -Path $zip -DestinationPath $tmp -Force

$dest = Join-Path $env:LOCALAPPDATA "arc-cli"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item (Join-Path $tmp "arc.exe") $dest -Force

# Add to PATH if not already there
$path = [Environment]::GetEnvironmentVariable("Path", "User")
if ($path -notlike "*$dest*") {
    [Environment]::SetEnvironmentVariable("Path", "$path;$dest", "User")
    $env:Path += ";$dest"
}

Remove-Item $tmp -Recurse -Force
Write-Host "ARC CLI $version installed to $dest\arc.exe"
arc --version
