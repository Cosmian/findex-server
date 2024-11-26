# Quick start

Multiple options are available to run the Findex server, including using Docker, pre-built binaries, or building from source.

!!! warning
    No authentication is configured for quick start. This is not recommended for production use.

## Docker

The quickest way to get started with Findex server is to use the Docker image. To run the server binary on `http://localhost:6668` that stores its data
in a Redis server, run the following command:

```sh
docker compose -f docker-compose-quick-start.yml up
```

where `docker-compose-quick-start.yml` is the following:

```yaml
services:
  redis:
    container_name: redis
    image: redis:latest
    ports:
      - 6379:6379
  findex-server:
    container_name: findex-server
    image: ghcr.io/cosmian/findex-server:0.1.0
    ports:
      - 6668:6668
    environment:
      FINDEX_SERVER_DATABASE_TYPE: redis
      FINDEX_SERVER_DATABASE_URL: redis://redis:6379
      FINDEX_SERVER_CLEAR_DATABASE: true

```

## Pre-built binaries

An other option include running the server binary directly or building it from source: pre-built binaries [are available](https://package.cosmian.com/findex-server/0.1.0/) for Linux, MacOS, and Windows.

First, run the Redis server independently:

```sh
docker run -d -p 6379:6379 redis
```

Then, download the binary for your platform and run it:

```sh
wget https://package.cosmian.com/findex-server/0.1.0/ubuntu_24_04-release.zip
unzip ubuntu_24_04-release.zip
./ubuntu_24_04-release/cosmian_findex_server -- --database-url redis://localhost:6379 --database-type redis
```

The server should now be running on `http://localhost:6668`.

## From source

To build the server from source, clone the repository and run the following commands:

```sh
git clone https://github.com/Cosmian/findex-server.git
cd findex-server
cargo build
```

First, run the Redis server independently:

```sh
docker run -d -p 6379:6379 redis
```

Then, run the server:

```sh
cargo run --bin cosmian_findex_server -- --database-url redis://localhost:6379 --database-type redis
```
