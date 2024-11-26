# Cosmian Findex server

The Cosmian Findex is a high-performance, [**open-source**](https://github.com/Cosmian/findex-server), server application
written in [**Rust**](https://www.rust-lang.org/) that provides a REST API.

## What is Findex?

Findex is a cryptographic protocol designed to make search queries on an untrusted cloud server securely. Findex is concurrent and database-independent, allowing large database indexes to be outsourced securely without compromising usability.

Findex aims to solve the following problem:

!!! important
    How to securely recover the _location_ of an encrypted data matching a given _keyword_?

Findex has been published as a scientific paper in the IACR ePrint archive: <https://eprint.iacr.org/2024/1541>.

## What is Findex server?

![Architecture client server](../images/Findex.svg)

Findex server is a high-performance, open-source server application written in Rust that implements the Findex protocol and offers a REST API to store encrypted indexes and perform search queries on them. The server is designed to be used in conjunction with the Findex CLI, a command-line interface that allows users to interact with the server.

### Attacking Model

The attacking model for Findex assumes that the cloud server is untrusted and may attempt to infer information from the encrypted indexes and search queries. However, the server is considered honest-but-curious, meaning it will follow the protocol correctly but will try to learn as much as possible from the data it processes. Findex is designed to protect against such adversaries by ensuring that no useful information about the plaintext data or search queries is leaked.

!!! important
    Basically, the server does not know how are encrypted the indexes nor the datasets they contain. It can only perform search queries on the encrypted indexes and return the results to the client.

## Findex server

### Client-Side Encryption

Client-side encryption is a crucial aspect of the Findex protocol, ensuring that data remains secure even when stored on an untrusted server. In this model, all encryption and decryption operations are performed on the client side, meaning that the server only ever sees encrypted data and cannot infer any useful information from it.

```mermaid
architecture-beta
    service user(disk)[Client side encryption]

    group untrusted(cloud)[Cloud]

    service server(server)[Findex Server] in untrusted
    service db(database)[Database] in untrusted
    service kms(server)[KMS] in untrusted
    service db2(database)[Keys] in untrusted

    user:R -- L:server
    server:R <--> L:db
    user:R -- L:kms
    kms:R <--> L:db2
```

#### How It Works

1. **Data Encryption**: Before sending any data to the Findex server, the client encrypts the data using a secure encryption algorithm. This ensures that the plaintext data is never exposed to the server.

2. **Index Encryption**: Similarly, the indexes used for search queries are also encrypted on the client side. This prevents the server from learning anything about the content of the indexes.

3. **Search Queries**: When performing a search query, the client encrypts the search keyword and sends the encrypted query to the server. The server processes the query on the encrypted indexes and returns the encrypted results to the client.

4. **Data Decryption**: Upon receiving the encrypted results from the server, the client decrypts the data to obtain the plaintext results.

#### Benefits

- **Data Privacy**: Since the server only handles encrypted data, it cannot access the plaintext information, ensuring data privacy.
- **Security**: Client-side encryption protects against potential data breaches on the server, as the encrypted data would be useless to an attacker without the decryption keys.
- **Control**: Users retain full control over their encryption keys and can manage their own security policies.

By leveraging client-side encryption, Findex ensures that sensitive data remains secure and private, even when stored and processed on an untrusted server.

The Findex server, written in Rust and using the Actix-web framework, is a REST-API server used to store encrypted indexes and perform search queries on them. It is designed to be used in conjunction with the Findex CLI, which is a command-line interface that allows users to interact with the server.

### Features

The server offers a REST API with the following functionalities:

- **User authentication**
- **User access authorization**
- **Store and retrieve encrypted indexes as Findex requirements**
- **Store and retrieve the encrypted version of the data that has been indexed**