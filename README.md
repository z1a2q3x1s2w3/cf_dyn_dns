# Cloudflare Dynamic DNS Updater

This Rust application updates the A record of a specified domain on Cloudflare with the WAN IP address of the machine running the application. It uses Cloudflare's API to perform the DNS record update.

## Features

- Fetches the current WAN IP address.
- Retrieves the current DNS A record for a specified domain.
- Updates the DNS A record if the WAN IP address has changed.

## Prerequisites

- Rust installed on your machine. You can download it from [rust-lang.org](https://www.rust-lang.org/).
- A Cloudflare account with API access.

## Installation

1. Clone the repository:
    ```sh
    git clone https://github.com/z1a2q3x1s2w3/cf_dyn_dns.git
    cd cf_dyn_dns
    ```

2. Install the required Rust crates by running:
    ```sh
    cargo build
    ```

3. Copy and update the `.env` file in the root directory of the project:
    ```sh
    cp .env.example .env
    vi .env
    ```

    - `DOMAIN`: The domain name you want to update.
    - `ZONE_ID`: The zone ID for your domain in Cloudflare.
    - `TOKEN`: Your Cloudflare API token with permission to edit DNS records.

## Usage

To run the application, use the following command:
```sh
cargo run
```

The application will:

    - Fetch the current WAN IP address.
    - Retrieve the current DNS A record for the specified domain.
    - Update the DNS A record on Cloudflare if the WAN IP address has changed.

## Logging

Logging is configured using log4rs. Ensure you have a log4rs.yaml configuration file in the root directory. Adjust the logging configuration as needed.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any changes.

## Acknowledgements

    - reqwest: A simple HTTP and HTTPS client for Rust.
    - serde_json: A JSON serialization and deserialization library for Rust.
    - dotenv: A library for loading environment variables from a .env file.
    - log: A logging facade for Rust.
    - log4rs: A highly configurable logging framework for Rust.

## Troubleshooting

If you encounter any issues, please check the logs for detailed error messages. Ensure that all environment variables are correctly set in the .env file. If the problem persists, feel free to open an issue on GitHub.

## Contact

For any questions or feedback, please contact z1a2q3x1s2w3.
