# Usage

```sh
Cosmian Findex server

Usage: cosmian_findex_server [OPTIONS]

Options:
      --database-type <DATABASE_TYPE>
          The database type of the Findex server
          - sqlite: `SQLite`. The data will be stored at the `sqlite_path`
            directory
          - redis: Redis database. The Redis url must be provided [env: FINDEX_SERVER_DATABASE_TYPE=] [possible values: redis]
      --database-url <DATABASE_URL>
          The url of the database for findex-redis [env: FINDEX_SERVER_DATABASE_URL=] [default: redis://localhost:6379]
      --sqlite-path <SQLITE_PATH>
          The directory path of the sqlite or sqlite-enc [env: FINDEX_SERVER_SQLITE_PATH=] [default: ./sqlite-data]
      --clear-database
          Clear the database on start.
          WARNING: This will delete ALL the data in the database [env: FINDEX_SERVER_CLEAR_DATABASE=]
      --port <PORT>
          The Findex server port [env: FINDEX_SERVER_PORT=] [default: 6668]
      --hostname <HOSTNAME>
          The Findex server hostname [env: FINDEX_SERVER_HOSTNAME=] [default: 0.0.0.0]
      --https-p12-file <HTTPS_P12_FILE>
          The Findex server optional PKCS#12 Certificates and Key file. If provided, this will start the server in HTTPS mode [env: FINDEX_SERVER_HTTPS_P12_FILE=]
      --https-p12-password <HTTPS_P12_PASSWORD>
          The password to open the PKCS#12 Certificates and Key file [env: FINDEX_SERVER_HTTPS_P12_PASSWORD=]
      --authority-cert-file <AUTHORITY_CERT_FILE>
          The server optional authority X509 certificate in PEM format used to validate the client certificate presented for authentication. If provided, this will require clients to present a certificate signed by this authority for authentication. The server must run in TLS mode for this to be used [env: FINDEX_SERVER_AUTHORITY_CERT_FILE=]
      --jwt-issuer-uri <JWT_ISSUER_URI>...
          The issuer URI of the JWT token [env: FINDEX_SERVER_JWT_ISSUER_URI=]
      --jwks-uri <JWKS_URI>...
          The JWKS (Json Web Key Set) URI of the JWT token [env: FINDEX_SERVER_JWKS_URI=]
      --jwt-audience <JWT_AUDIENCE>...
          The audience of the JWT token [env: FINDEX_SERVER_JST_AUDIENCE=]
      --default-username <DEFAULT_USERNAME>
          The default username to use when no authentication method is provided [env: FINDEX_SERVER_DEFAULT_USERNAME=] [default: admin]
      --force-default-username
          When an authentication method is provided, perform the authentication but always use the default username instead of the one provided by the authentication method [env: FINDEX_SERVER_FORCE_DEFAULT_USERNAME=]
  -h, --help
          Print help (see more with '--help')
  -V, --version
          Print version
```
