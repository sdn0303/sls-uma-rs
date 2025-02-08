# Sample Serverless User Management Auth Service Rust

This project is a sample user management and authentication system that adopts Rust and serverless architecture.

## Architecture

See [`template.yaml`](./template.yaml)

## Setup (AppleSilicon)

### Install the Rust toolchain

If your First time to install the Rust toolchain, you can use the following command.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

- Doc: [https://www.rust-lang.org/ja/learn/get-started](https://www.rust-lang.org/ja/learn/get-started)
- Doc: [Cargo getting started](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- Doc: [Rust The Book](https://doc.rust-lang.org/book/)

After installing the Rust toolchain, check the version.

This project uses the following version.

```bash
rustup -V
rustup 1.27.1 (2024-04-24)
info: This is the version for the rustup toolchain manager, not the rustc compiler.
info: The currently active `rustc` version is `rustc 1.80.1 (3f5fd8dd4 2024-08-06)`
```

```bash
cargo version
cargo 1.80.1 (376290515 2024-07-16)
```

### Install the cargo-make

```bash
cargo install --force cargo-make
```

- Github: [https://github.com/sagiegurari/cargo-make](https://github.com/sagiegurari/cargo-make)

### Install the cargo-lambda

```bash
brew tap cargo-lambda/cargo-lambda
brew install cargo-lambda
```

- Doc: [https://www.cargo-lambda.info/guide/installation.html](https://www.cargo-lambda.info/guide/installation.html)

### Install the compiler and the target for the cross-compilation

```bash
brew install filosottile/musl-cross/musl-cross
```

```bash
rustup target add aarch64-unknown-linux-musl
```

*If your build fails with the openssl related error, try to set the environment variables like this*

```bash
export OPENSSL_DIR=$(brew --prefix openssl@3)
export OPENSSL_LIB_DIR=$(brew --prefix openssl@3)/lib
export OPENSSL_INCLUDE_DIR=$(brew --prefix openssl@3)/include
export PKG_CONFIG_PATH=$(brew --prefix openssl@3)/lib/pkgconfig
```

### Install the SAM CLI

Set AWS credentials in you local.

```bash
brew install aws-sam-cli
```

## Commands

This project uses the `cargo-make` for the build and the deployment.

More information, see the [`Makefile.toml`](./Makefile.toml)
and [https://sagiegurari.github.io/cargo-make/](https://sagiegurari.github.io/cargo-make/).

### Build the Lambda and templates

```bash
cargo make build-all
```

```bash
sam build --profile { your profile }
```

### Deploy SAM

```bash
sam deploy --profile { your profile }
```

## API Endpoints

```text
POST   /signup
POST   /login
POST   /tokens/refresh
GET    /tokens/validate
GET    /organizations/{organizationId}/users
POST   /organizations/{organizationId}/users
GET    /organizations/{organizationId}/users/{userId}
PUT    /organizations/{organizationId}/users/{userId}
DELETE /organizations/{organizationId}/users/{userId}
```
