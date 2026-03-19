$lines = Get-Content Cargo.toml
$lines[0..110] | Set-Content Cargo.toml
Add-Content Cargo.toml @"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
dbg_macro = "deny"

[workspace.lints.rust]
dead_code = "allow"
unused_imports = "warn"
"@

$crates = Get-ChildItem -Path "crates" -Directory
foreach ($crate in $crates) {
    $cargoPath = Join-Path $crate.FullName "Cargo.toml"
    if (Test-Path $cargoPath) {
        $content = Get-Content $cargoPath -Raw
        if ($content -notmatch '\[lints\]') {
            Add-Content -Path $cargoPath -Value "`n[lints]`nworkspace = true"
            Write-Host "Added [lints] to $($crate.Name)"
        }
    }
}
