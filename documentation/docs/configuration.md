
# Configuration file

By default, the server expects to find:

- a TOML configuration file in `/etc/cosmian/findex_server.toml`.
- or an environment variable `COSMIAN_FINDEX_SERVER_CONF` that contains the path to the configuration file.
- otherwise, the server will parse the arguments passed in command line.

## Example without authentication

```toml
default_username = "admin"
force_default_username = false

[db]
database_type = "Redis"
database_url = "redis://localhost:6379"
clear_database = false

[http]
port = 6668
hostname = "0.0.0.0"
```

## Example with X509 authentication

```toml
default_username = "admin"
force_default_username = false

[db]
database_type = "Redis"
database_url = "redis://localhost:6379"
clear_database = true

[http]
port = 6660
hostname = "0.0.0.0"
https_p12_file = "/etc/cosmian/certificates/server/findex.server.acme.com.p12"
https_p12_password = "password"
authority_cert_file = "/etc/cosmian/certificates/server/ca.crt"
```

## Example with OpenID authentication

```toml
default_username = "admin"
force_default_username = false

[db]
database_type = "Redis"
database_url = "redis://localhost:6379"
clear_database = false

[http]
port = 6668
hostname = "0.0.0.0"

[auth]
jwt_issuer_uri = "eyJhbGciOiJSUzI1NiIsInR5cCI...ydoDOsmYhWTEgf5w"
```
