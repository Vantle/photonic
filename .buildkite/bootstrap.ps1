$ErrorActionPreference = 'Stop'

$platform = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    'Arm64' { 'arm64' }
    'X64' { 'amd64' }
    default { throw 'Unsupported agent platform' }
}

$checksum = switch ($platform) {
    'arm64' { '85ba3d92a8bdcbecc06657b8c0ae30f4307b552d601d9d6246f8a98aec36c346' }
    'amd64' { 'b9d65a1f7c2d7af885a96a4fd5aa36b40fb41816d30944390569eef908bdc954' }
}

$directory = Join-Path $env:BUILDKITE_BUILD_CHECKOUT_PATH '.cache/bootstrap/1.28.1'
$binary = Join-Path $directory 'bazel.exe'
New-Item -ItemType Directory -Force -Path $directory | Out-Null
if (!(Test-Path $binary)) {
    $download = Join-Path $directory ([System.Guid]::NewGuid().ToString())
    Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/bazelbuild/bazelisk/releases/download/v1.28.1/bazelisk-windows-$platform.exe" -OutFile $download
    if ((Get-FileHash $download -Algorithm SHA256).Hash -ne $checksum) {
        Remove-Item $download
        throw 'Bazelisk checksum mismatch'
    }
    Move-Item $download $binary -Force
}
if ((Get-FileHash $binary -Algorithm SHA256).Hash -ne $checksum) {
    throw 'Bazelisk checksum mismatch'
}

$checkout = $env:BUILDKITE_BUILD_CHECKOUT_PATH.Replace('\', '/')
@(
    "build --disk_cache=`"$checkout/.cache/action`""
    "common --repository_cache=`"$checkout/.cache/repository`""
) | Set-Content -Encoding Ascii user.bazelrc
