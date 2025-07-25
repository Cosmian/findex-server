$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $true # might be true by default

function BuildProject {
    param (
        [Parameter(Mandatory = $true)]
        [ValidateSet("debug", "release")]
        [string]$BuildType
    )

    # Add target
    rustup target add x86_64-pc-windows-msvc

    $env:OPENSSL_DIR = "$env:VCPKG_INSTALLATION_ROOT\packages\openssl_x64-windows-static"
    Get-ChildItem -Recurse $env:OPENSSL_DIR

    # Build `server`
    $env:RUST_LOG = "cosmian_findex_cli=error,cosmian_findex_server=error,test_findex_server=error"
    $env:FINDEX_TEST_DB = "sqlite-findex"
    if ($BuildType -eq "release") {
        cargo build --features "non-fips" -p cosmian_findex_server -p cosmian_findex_cli --release --target x86_64-pc-windows-msvc
        cargo  test --features "non-fips" -p cosmian_findex_server -p cosmian_findex_cli --target x86_64-pc-windows-msvc -- --nocapture --skip kms --skip hsm --skip redis
    }
    else {
        cargo build --features "non-fips" -p cosmian_findex_server -p cosmian_findex_cli --target x86_64-pc-windows-msvc
        cargo  test --features "non-fips" -p cosmian_findex_server -p cosmian_findex_cli --target x86_64-pc-windows-msvc -- --nocapture --skip kms --skip hsm --skip redis
    }

    # Check dynamic links
    $output = & "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.29.30133\bin\HostX64\x64\dumpbin.exe" /dependents target\x86_64-pc-windows-msvc\$BuildType\cosmian_findex_server.exe | Select-String "libcrypto"
    if ($output) {
        throw "OpenSSL (libcrypto) found in dynamic dependencies. Error: $output"
    }

    exit 0
}


# Example usage:
# BuildProject -BuildType debug
# BuildProject -BuildType release
