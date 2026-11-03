$ErrorActionPreference = "Stop"
Push-Location "$PSScriptRoot/.."
try {
    python scripts/build_local.py --platform windows --arch x64
    if ($LASTEXITCODE -ne 0) { throw "Build failed ($LASTEXITCODE)" }
} finally { Pop-Location }
