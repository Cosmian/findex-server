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

    # Build `cosmian`
    if ($BuildType -eq "release") {
        cargo build -p cosmian_cli --release --target x86_64-pc-windows-msvc
    }
    else {
        cargo build -p cosmian_cli --target x86_64-pc-windows-msvc
    }

    # Check dynamic links
    $output = & "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.29.30133\bin\HostX64\x64\dumpbin.exe" /dependents target\x86_64-pc-windows-msvc\$BuildType\cosmian.exe | Select-String "libcrypto"
    if ($output) {
        throw "OpenSSL (libcrypto) found in dynamic dependencies. Error: $output"
    }

    # Build `server`
    if ($BuildType -eq "release") {
        cargo build -p cosmian_findex_server --release --target x86_64-pc-windows-msvc
        cargo test  -p cosmian_findex_server --release --target x86_64-pc-windows-msvc -- --nocapture --skip test_findex --skip test_all_authentications --skip test_server_auth_matrix --skip test_datasets --skip test_permissions
    }
    else {
        cargo build -p cosmian_findex_server --target x86_64-pc-windows-msvc
        cargo test  -p cosmian_findex_server --target x86_64-pc-windows-msvc -- --nocapture --skip test_findex --skip test_all_authentications --skip test_server_auth_matrix --skip test_datasets --skip test_permissions
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
