# Quick start

Multiple options are available to run the Findex server, including using Docker, pre-built binaries, or building from source.

!!! warning
    No authentication is configured for quick start. This is not recommended for production use.

=== "Docker"

    The quickest way to get started with Findex server is to use the Docker image.
    To run the server binary on `http://localhost:6668` that stores its data
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
        image: ghcr.io/cosmian/findex-server:latest
        ports:
          - 6668:6668
        environment:
          FINDEX_SERVER_DATABASE_TYPE: redis
          FINDEX_SERVER_DATABASE_URL: redis://redis:6379
          FINDEX_SERVER_CLEAR_DATABASE: true
    ```

=== "Ubuntu 24.04"

    An other option include running the server binary directly installing the
    Debian package [available here](https://package.cosmian.com/findex-server/0.4.11/ubuntu-24.04/).

    First, run the Redis server independently:

    ```sh
    docker run -d -p 6379:6379 redis
    ```

    Then, download package and install it:

    ```console title="On local machine"
    sudo apt update && sudo apt install -y wget
    wget https://package.cosmian.com/findex-server/0.4.11/ubuntu-24.04/cosmian-findex-server_0.4.11-1_amd64.deb
    sudo apt install ./cosmian-findex-server_0.4.11-1_amd64.deb
    cosmian_findex_server --version
    ```

    The server should now be running on `http://localhost:6668`.

=== "RHEL 9"

    An other option include running the server binary directly installing the
    Debian package [available here](https://package.cosmian.com/findex-server/0.4.11/rhel9/).

    First, run the Redis server independently:

    ```sh
    docker run -d -p 6379:6379 redis
    ```

    Then, download package and install it:

    ```console title="On local machine"
    sudo dnf update && dnf install -y wget
    wget https://package.cosmian.com/findex-server/0.4.11/rhel9/cosmian_findex_server-0.4.11-1.x86_64.rpm
    sudo dnf install ./cosmian_findex_server-0.4.11-1.x86_64.rpm
    cosmian_findex_server --version
    ```

    The server should now be running on `http://localhost:6668`.

=== "MacOS"

    On ARM MacOS, download the build archive and extract it:

    ```console title="On local machine"
    wget https://package.cosmian.com/findex-server/0.4.11/macos_arm-release.zip
    unzip macos_arm-release.zip
    cp ./macos_arm-release/cosmian_findex_server /usr/local/bin/
    chmod u+x /usr/local/bin/cosmian_findex_server
    cosmian_findex_server --version
    ```

=== "Windows"

    On Windows, download the build archive:

    ```console title="Build archive"
     https://package.cosmian.com/findex-server/0.4.11/windows-release.zip
    ```

    Extract the cosmian_findex_server from:

    ```console title="cosmian_findex_server for Windows"
    /windows-release/cosmian_findex_server.exe
    ```

    Copy it to a folder in your PATH and run it:

    ```console title="On local machine"
    cosmian_findex_server --version
    ```

=== "From source"

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

## Configuration

Please refer to the [configuration documentation](./configuration.md) for more
information on how to configure the Findex server.
