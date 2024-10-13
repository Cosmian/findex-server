use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use cloudproof::reexport::crypto_core::bytes_ser_de::{Deserializer, Serializer};
use serde::{de::DeserializeOwned, Serialize};

use crate::{error::ClientError, ClientResultHelper};

/// Read all bytes from a file
pub fn read_bytes_from_file(file: &impl AsRef<Path>) -> Result<Vec<u8>, ClientError> {
    let mut buffer = Vec::new();
    File::open(file)
        .with_context(|| format!("could not open the file {}", file.as_ref().display()))?
        .read_to_end(&mut buffer)
        .with_context(|| format!("could not read the file {}", file.as_ref().display()))?;

    Ok(buffer)
}

/// Read an object T from a JSON file
pub fn read_from_json_file<T>(file: &impl AsRef<Path>) -> Result<T, ClientError>
where
    T: DeserializeOwned,
{
    let buffer = read_bytes_from_file(file)?;
    serde_json::from_slice::<T>(&buffer)
        .with_context(|| "failed parsing the object from the json file")
}

/// Write all bytes to a file
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

/// Read all bytes from multiple files and serialize them
/// into a unique vector using LEB128 serialization (bulk mode)
pub fn read_bytes_from_files_to_bulk(input_files: &[PathBuf]) -> Result<Vec<u8>, ClientError> {
    let mut ser = Serializer::new();

    // number of files to decrypt
    let nb_input_files = u64::try_from(input_files.len()).map_err(|_| {
        ClientError::Conversion(format!(
            "number of input files is too big for architecture: {} bytes",
            input_files.len()
        ))
    })?;
    ser.write_leb128_u64(nb_input_files)?;

    input_files.iter().try_for_each(|input_file| {
        let content = read_bytes_from_file(input_file)?;
        ser.write_vec(&content)?;
        Ok::<_, ClientError>(())
    })?;

    Ok(ser.finalize().to_vec())
}

/// Write bulk decrypted data
///
/// Bulk data is compound of multiple chunks of data.
/// Sizes are written using LEB-128 serialization.
///
/// Each chunk of plaintext data is written to its own file.
pub fn write_bulk_decrypted_data(
    plaintext: &[u8],
    input_files: &[PathBuf],
    output_file: Option<&PathBuf>,
) -> Result<(), ClientError> {
    let mut de = Deserializer::new(plaintext);

    // number of decrypted chunks
    let nb_chunks = {
        let len = de.read_leb128_u64()?;
        usize::try_from(len).map_err(|_| {
            ClientError::Conversion(format!(
                "size of vector is too big for architecture: {len} bytes",
            ))
        })?
    };

    (0..nb_chunks).try_for_each(|idx| {
        // get chunk of data from slice
        let chunk_data = de.read_vec_as_ref()?;

        // Write plaintext data to its own file
        // Reuse input file names if there are multiple inputs (and ignore `output_file`)
        let input_file = &input_files[idx];
        let output_file = match output_file {
            Some(output_file) if nb_chunks > 1 => {
                let file_name = input_file.file_name().ok_or_else(|| {
                    ClientError::Conversion(format!(
                        "cannot get file name from input file {input_file:?}",
                    ))
                })?;
                output_file.join(PathBuf::from(file_name).with_extension("plain"))
            }
            _ => output_file.map_or_else(
                || input_file.with_extension("plain"),
                std::clone::Clone::clone,
            ),
        };

        write_bytes_to_file(chunk_data, &output_file)?;

        tracing::info!("The decrypted file is available at {output_file:?}");
        Ok(())
    })
}

/// Write bulk encrypted data
///
/// Bulk data is compound of multiple chunks of data.
/// Sizes are written using LEB-128 serialization.
///
/// Each chunk of data:
/// - is compound of encrypted header + encrypted data
/// - is written to its own file.
pub fn write_bulk_encrypted_data(
    plaintext: &[u8],
    input_files: &[PathBuf],
    output_file: Option<&PathBuf>,
) -> Result<(), ClientError> {
    let mut de = Deserializer::new(plaintext);

    // number of encrypted chunks
    let nb_chunks = {
        let len = de.read_leb128_u64()?;
        usize::try_from(len).map_err(|_| {
            ClientError::Conversion(format!(
                "size of vector is too big for architecture: {len} bytes",
            ))
        })?
    };

    (0..nb_chunks).try_for_each(|idx| {
        // get chunk of data from slice
        let chunk_data = de.read_vec_as_ref()?;

        // Write encrypted data to its own file
        // Reuse input file names if there are multiple inputs (and ignore `output_file`)
        let input_file = &input_files[idx];
        let output_file = match output_file {
            Some(output_file) if nb_chunks > 1 => {
                let file_name = input_file.file_name().ok_or_else(|| {
                    ClientError::Conversion(format!(
                        "cannot get file name from input file {input_file:?}",
                    ))
                })?;
                output_file.join(PathBuf::from(file_name).with_extension("enc"))
            }
            _ => output_file.map_or_else(
                || input_file.with_extension("enc"),
                std::clone::Clone::clone,
            ),
        };

        write_bytes_to_file(chunk_data, &output_file)?;

        tracing::info!("The encrypted file is available at {output_file:?}");
        Ok(())
    })
}
