# Move Binding

Move Binding is a Rust library that provides a way to interact with Sui Move packages on-chain. It reads Move packages from the Sui blockchain and generates corresponding Rust structs and function entry points, allowing for seamless integration between Move and Rust.

## Features
- Reads Sui Move packages directly from the Sui blockchain.
- Generates Rust representations of Move objects.
- Provides Rust function entry points for Move contract interactions. -- Coming soon
- Facilitates seamless integration between Move smart contracts and Rust applications. -- Coming soon

## Installation
To use Move Binding in your project, add the following dependency to your `Cargo.toml`:

```toml
[dependencies]
move-binding-derive = { git = "https://github.com/MystenLabs/move-binding" }
move-types = { git = "https://github.com/MystenLabs/move-binding" }
```

## Usage

### Import a Move Package from Sui
```rust
use std::str::FromStr;
use sui_client::Client;
use sui_sdk_types::{Address, ObjectData};

use crate::bridge::bridge::BridgeInner;
use crate::sui::dynamic_field::Field;
use move_binding_derive::move_contract;

move_contract! {alias = "sui", package = "0x2"}
move_contract! {alias = "bridge", package = "0xb", deps = [crate::sui]}

#[tokio::main]
async fn main() {
    let client = Client::new("https://sui-mainnet.mystenlabs.com/graphql").unwrap();
    let bridge_obj = client
        .object(
            Address::from_str(
                "0x00ba8458097a879607d609817a05599dc3e9e73ce942f97d4f1262605a8bf0fc".into(),
            )
                .unwrap(),
            None,
        )
        .await
        .unwrap()
        .unwrap();

    if let ObjectData::Struct(o) = bridge_obj.data() {
        let bridge: Field<u64, BridgeInner> = bcs::from_bytes(o.contents()).unwrap();
        println!("Deserialized Bridge object: {:?}", bridge);
    }
}
```

## Development
Clone the repository and build the project:

```sh
git clone https://github.com/MystenLabs/move-binding.git
cd move-binding
cargo build
```

To run tests:
```sh
cargo test
```

## Contributing
Contributions are welcome! Please open an issue or submit a pull request if you have any improvements or bug fixes.

## License
This project is licensed under the Apache 2.0 License.

---
