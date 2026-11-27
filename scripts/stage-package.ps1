$ErrorActionPreference = 'Stop'

$root = (Resolve-Path -LiteralPath "$PSScriptRoot/..").Path
$export = Join-Path $root 'build/qpm2-export'
if (Test-Path -LiteralPath $export) {
    $existing = Get-Item -LiteralPath $export -Force
    if ($existing.FullName -ne $export -or $existing.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "Unexpected header staging path: $($existing.FullName)"
    }
    Remove-Item -LiteralPath $export -Recurse -Force
}

# Keep the public paper2_scotland2/shared/... include paths in the QPKG.
$headers = Join-Path $export 'shared'
$utfcpp = Join-Path $headers 'utfcpp'
New-Item -ItemType Directory -Path $utfcpp -Force | Out-Null
Copy-Item -Path "$root/shared/*.h", "$root/shared/*.hpp", "$root/shared/feature" -Destination $headers -Recurse
Copy-Item -LiteralPath "$root/shared/utfcpp/source", "$root/shared/utfcpp/LICENSE" -Destination $utfcpp -Recurse

# QPM reads outBinaries from extern/build, while qpm-action uploads from the project root.
$binary = 'build/arm64-v8a/libpaper2_scotland2.so'
$binaryStage = Join-Path $root 'extern/build/build/arm64-v8a'
New-Item -ItemType Directory -Path $binaryStage -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $root $binary) -Destination $binaryStage -Force
