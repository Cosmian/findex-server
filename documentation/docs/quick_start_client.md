# Using the Cosmian CLI

The [Cosmian CLI](../cosmian_cli/index.md) allows to interact both with the
**KMS server** and **Findex server**. Then you can realize the following operations
illustrated in [encrypted database](./database.md#how-to-securely-index-new-data).

As prerequisites, you need to have:

- the Cosmian CLI installed on your machine. You can download the latest version of the Cosmian CLI from the [Cosmian package website](https://package.cosmian.com/cli).
- a running KMS server: follow the instructions in the [quick start guide](../key_management_system/installation/installation_getting_started.md/).
- a running Findex server: follow the instructions in the [quick start guide](./quick_start.md).

Depending on the authentication of these 2 servers, the configuration will have to be adapted. Refer to the [configuration documentation](../cosmian_cli/configuration.md) for more information.

As a summary, below are described the steps:

- to encrypt and store a dataset while indexing it.
- to search a keyword and decrypt the corresponding result.

## Encrypt and store a dataset while indexing it

The following steps allow to encrypt a data using a KEM-DEM crypto-system where:

- the Data Encryption Key (DEK) is encrypted with a KMS server (with RFC-5649 wrapping key standard) using a Key Encryption Key (KEK)
- the encrypted DEK is stored with the data encrypted using AES-256-GCM.

Generate the Key Encryption Key:

```sh
cosmian kms sym keys create

The symmetric key was successfully generated.
            Unique identifier: 55629c83-5184-4e54-9839-9b686a6f2850
```

Generate the Seed Key that will be used to encrypt locally the indexes:

```sh
cosmian kms sym keys create

The symmetric key was successfully generated.
            Unique identifier: fbf2a111-ae11-4231-9985-c0e3b140caeb
```

Then create a dedicated `index id` where all data will be stored:

```sh
cosmian findex-server permissions create

[admin] New admin permission successfully created on index: 13348510-75cd-436e-a9ff-60de66cac0d0
```

Then encrypt and index the following small dataset:

```csv
city,region,country,population
Southborough,MA,United States,9686
Northbridge,MA,United States,14061
Westborough,MA,United States,29313
Marlborough,MA,United States,38334
Springfield,MA,United States,152227
Springfield,MO,United States,150443
Springfield,NJ,United States,14976
Springfield,OH,United States,64325
Springfield,OR,United States,56032
Concord,NH,United States,42605
```

Using this command:

```sh
cosmian findex-server encrypt-and-index --seed-key-id fbf2a111-ae11-4231-9985-c0e3b140caeb --index-id 13348510-75cd-436e-a9ff-60de66cac0d0 --csv test_data/datasets/smallpop.csv --kek-id 55629c83-5184-4e54-9839-9b686a6f2850

Data behind those UUIDS were encrypted and indexed: UUID: 7059592d-9cd7-46d6-9e4d-b26436430942
UUID: d9eee59c-f9df-4edd-97bc-ba5952ce63af
UUID: 5b044b87-bced-424c-9dac-f25550c88c20
UUID: 863f39c5-bd6f-4685-97a2-07de8aa67c41
UUID: 90d05319-15c7-4c39-8176-85057c915b7b
UUID: dfe699ab-493d-49fc-b19c-9918376b2aa5
UUID: 61fcac44-0e91-470c-95f5-496f1b67389a
UUID: c6ab6f96-bba5-478b-be7f-2526e5b82e41
UUID: 3329fe2a-06c5-4904-aba7-4a9faf0e0876
UUID: f556fd71-7ef1-46ca-bf0f-30d92c055b44
```

## Search a keyword

To search a keyword, find the corresponding UUIDs, get the values of these UUIDs and decrypt them (if any match is found), just proceed as follow:

```sh

cosmian findex-server search-and-decrypt --seed-key-id fbf2a111-ae11-4231-9985-c0e3b140caeb --index-id 13348510-75cd-436e-a9ff-60de66cac0d0 --kek-id 55629c83-5184-4e54-9839-9b686a6f2850 --keyword Southborough

Decrypted record: SouthboroughMAUnited States9686
```

## Configuration

Please refer to the [configuration documentation](../cosmian_cli/configuration.md) for more information on how to configure the Cosmian CLI.
