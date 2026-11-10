$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$flags = @(
    "--remap-path-prefix=$($env:USERPROFILE)=user",
    "--remap-path-prefix=$root=ootrr",
    "-C",
    "link-arg=/PDBALTPATH:%_PDB%"
)
Push-Location -LiteralPath $root
try {
    $env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]0x1f
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "build failed" }
} finally {
    Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
    Pop-Location
}
