use crate::{
    error::{result::ClientResult, ClientError},
    ClientResultHelper,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

/// Read all bytes from a file
/// # Errors
/// It returns an error if the file cannot be opened or read
pub fn read_bytes_from_file(file: &impl AsRef<Path>) -> ClientResult<Vec<u8>> {
    let mut buffer = Vec::new();
    File::open(file)
        .with_context(|| format!("could not open the file {}", file.as_ref().display()))?
        .read_to_end(&mut buffer)
        .with_context(|| format!("could not read the file {}", file.as_ref().display()))?;

    Ok(buffer)
}

/// Read an object T from a JSON file
/// # Errors
/// It returns an error if the file cannot be opened or read
pub fn read_from_json_file<T>(file: &impl AsRef<Path>) -> Result<T, ClientError>
where
    T: DeserializeOwned,
{
    let buffer = read_bytes_from_file(file)?;
    serde_json::from_slice::<T>(&buffer)
        .with_context(|| "failed parsing the object from the json file")
}

/// Write all bytes to a file
/// # Errors
/// It returns an error if the file cannot be written
pub fn write_bytes_to_file(bytes: &[u8], file: &impl AsRef<Path>) -> Result<(), ClientError> {
    fs::write(file, bytes).with_context(|| {
        format!(
            "failed writing {} bytes to {:?}",
            bytes.len(),
            file.as_ref()
        )
    })
}

/// Write a JSON object to a file
/// # Errors
/// It returns an error if the file cannot be written
pub fn write_json_object_to_file<T>(
    json_object: &T,
    file: &impl AsRef<Path>,
) -> Result<(), ClientError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec::<T>(json_object)
        .with_context(|| "failed parsing the object from the json file")?;
    write_bytes_to_file(&bytes, file)
}

/// Write the decrypted data to a file
///
/// If no `output_file` is provided, then
/// it reuses the `input_file` name with the extension `plain`.
/// # Errors
/// It returns an error if the file cannot be written
pub fn write_single_decrypted_data(
    plaintext: &[u8],
    input_file: &Path,
    output_file: Option<&PathBuf>,
) -> Result<(), ClientError> {
    let output_file = output_file.map_or_else(
        || input_file.with_extension("plain"),
        std::clone::Clone::clone,
    );

    write_bytes_to_file(plaintext, &output_file)
        .with_context(|| "failed to write the decrypted file")?;

    tracing::info!("The decrypted file is available at {output_file:?}");
    Ok(())
}

/// Write the encrypted data to a file
///
/// If no `output_file` is provided, then
/// it reuses the `input_file` name with the extension `enc`.
/// # Errors
/// It returns an error if the file cannot be written
pub fn write_single_encrypted_data(
    encrypted_data: &[u8],
    input_file: &Path,
    output_file: Option<&PathBuf>,
) -> Result<(), ClientError> {
    // Write the encrypted file
    let output_file = output_file.map_or_else(
        || input_file.with_extension("enc"),
        std::clone::Clone::clone,
    );

    write_bytes_to_file(encrypted_data, &output_file)
        .with_context(|| "failed to write the encrypted file")?;

    tracing::info!("The encrypted file is available at {output_file:?}");
    Ok(())
}
