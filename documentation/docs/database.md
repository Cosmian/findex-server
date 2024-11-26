# Encrypted database

From the server's perspective, only encrypted data is received and stored as-is. The server does not know how the data is encrypted and cannot decrypt it.

The requirements database is a key-value store where the keys are unique identifiers (UIDs) and the values are the encrypted indexes or datasets.

In this scenario, the user is responsible for encrypting the data before sending it to the server. A hybrid encryption scheme is used, where the data is encrypted with a Data Encryption Key (DEK) and the DEK is encrypted with a Key Encryption Key (KEK).

User requires a Key Management System to encrypt the Data Encryption Key (DEK).

## How to securely index new data?

!!! info
    The user is already authenticated and has the `write` permission to a given index.

```mermaid
sequenceDiagram
  autonumber
  actor U as User
  participant K as Key Management System
  participant F as Findex server

  U->>K: User requests a Key Encryption Key (KEK)
  K->>U: Send an ID of the KEK
  U->>U: Generate an ephemeral Data Encryption Key (DEK)
  U->>K: Encrypt the DEK with the KEK (RFC5649)
  K->>U: Send the encrypted DEK (encapsulation)

  loop Read and encrypt locally the dataset
    U-->U: Read line by line the dataset
    U-->U: For each line, an unique identifier (UID) is generated
    U-->U: Each line is encrypted with the DEK (AES-256-GCM)
  end

  U->>F: Send all encrypted lines (and encapsulation) and corresponding UIDs to a given Index ID

  loop Index and encrypt locally the dataset
    U->>U: Index plaintext line by keywords resulting encrypted indexes
  end

  U->>F: Send encrypted indexes
```

!!! warning
    For now, only Redis database is supported.

## How to securely search indexed data?

!!! info
    The user is already authenticated and has the `write` permission to a given index.

```mermaid
sequenceDiagram
  autonumber
  actor U as User
  participant F as Findex server
  participant K as Key Management System

  U->>F: User does a search query by keywords
  F->>U: If data has been indexed with the given keywords, sends dataset UIDs
  U->>F: User requests the values of the dataset UIDs
  F->>U: Sends the encrypted values of the dataset UIDs

  loop Read the encrypted values
    U->>U: For each value, get the encrypted DEK (encapsulation)
    U->>K: Decrypt the DEK with the KEK
    K->>U: Send the decrypted DEK
    U->>U: Decrypt each value with the DEK
  end
```

### Store and retrieve encrypted indexes as Findex requirements

According the Findex REST client implementation found in [cloudproof_rust](https://github.com/Cosmian/cloudproof_rust), the server presents the following endpoints:

| Endpoint                             | Description                 |
| ------------------------------------ | --------------------------- |
| `/indexes/{index_id}/fetch_entries`  | retrieve encrypted indexes  |
| `/indexes/{index_id}/fetch_chains`   | retrieve encrypted indexes  |
| `/indexes/{index_id}/upsert_entries` | insert encrypted indexes    |
| `/indexes/{index_id}/insert_chains`  | insert encrypted indexes    |
| `/indexes/{index_id}/delete_entries` | delete encrypted indexes    |
| `/indexes/{index_id}/delete_chains`  | delete encrypted indexes    |
| `/indexes/{index_id}/dump_tokens`    | print the encrypted indexes |

The encryption is done by the client before sending the data to the server.

### Store and retrieve the encrypted version of the data that has been indexed

Findex server stores as it is the encrypted version of the data that has been indexed. The server presents the following endpoints:

| Endpoint                                    | Description                  |
| ------------------------------------------- | ---------------------------- |
| `/datasets/{index_id}/datasets_add_entries` | insert new encrypted entries |
| `/datasets/{index_id}/datasets_del_entries` | delete encrypted entries     |
| `/datasets/{index_id}/datasets_get_entries` | get encrypted entries        |

The encryption is done by the client before sending the data to the server.
