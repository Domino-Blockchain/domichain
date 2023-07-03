---
title: "Rust Program Quickstart"
description: "This quickstart guide will demonstrate how to quickly setup, build, and deploy your first Rust based Domichain program to the blockchain."
keywords:
  - rust
  - cargo
  - toml
  - program
  - tutorial
  - intro to solana development
  - blockchain developer
  - blockchain tutorial
  - web3 developer
---

Rust is the most common programming language to write Domichain programs with. This quickstart guide will demonstrate how to quickly setup, build, and deploy your first Rust based Domichain program to the blockchain.

> **NOTE: **
> This guide uses the Domichain CLI and assumes you have setup your local development environment. Checkout our [local development quickstart guide](./local.md) here to quickly get setup.

## What you will learn

- How to install the Rust language locally
- How to initialize a new Domichain Rust program
- How to code a basic Domichain program in Rust
- How to build and deploy your Rust program

## Install Rust and Cargo

To be able to compile Rust based Domichain programs, install the Rust language and Cargo (the Rust package manager) using [Rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Run your localhost validator

The Domichain CLI comes with the [test validator](../developing/test-validator.md) built in. This command line tool will allow you to run a full blockchain cluster on your machine.

```bash
solana-test-validator
```

> **PRO TIP:**
> Run the Domichain test validator in a new/separate terminal window that will remain open. This command line program must remain running for your localhost validator to remain online and ready for action.

Configure your Domichain CLI to use your localhost validator for all your future terminal commands and Domichain program deployment:

```bash
solana config set --url localhost
```

## Create a new Rust library with Cargo

Domichain programs written in Rust are _libraries_ which are compiled to [BPF bytecode](../developing/on-chain-programs/faq.md#berkeley-packet-filter-bpf) and saved in the `.so` format.

Initialize a new Rust library named `hello_world` via the Cargo command line:

```bash
cargo init hello_world --lib
cd hello_world
```

Add the `solana-program` crate to your new Rust library:

```bash
cargo add solana-program
```

Open your `Cargo.toml` file and add these required Rust library configuration settings, updating your project name as appropriate:

```toml
[lib]
name = "hello_world"
crate-type = ["cdylib", "lib"]
```

## Create your first Domichain program

The code for your Rust based Domichain program will live in your `src/lib.rs` file. Inside `src/lib.rs` you will be able to import your Rust crates and define your logic. Open your `src/lib.rs` file in your favorite editor.

At the top of `lib.rs`, import the `solana-program` crate and bring our needed items into the local namespace:

```rust
use domichain_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
};
```

Every Domichain program must define an `entrypoint` that tells the Domichain runtime where to start executing your on chain code. Your program's [entrypoint](../developing/on-chain-programs/developing-rust#program-entrypoint) should provide a public function named `process_instruction`:

```rust
// declare and export the program's entrypoint
entrypoint!(process_instruction);

// program entrypoint's implementation
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    // log a message to the blockchain
    msg!("Hello, world!");

    // gracefully exit the program
    Ok(())
}
```

Every on chain program should return the `Ok` [result enum](https://doc.rust-lang.org/std/result/) with a value of `()`. This tells the Domichain runtime that your program executed successfully without errors.

This program above will simply [log a message](../developing/on-chain-programs/debugging#logging) of "_Hello, world!_" to the blockchain cluster, then gracefully exit with `Ok(())`.

## Build your Rust program

Inside a terminal window, you can build your Domichain Rust program by running in the root of your project (i.e. the directory with your `Cargo.toml` file):

```bash
cargo build-bpf
```

> **NOTE:**
> After each time you build your Domichain program, the above command will output the build path of your compiled program's `.so` file and the default keyfile that will be used for the program's address.
> `cargo build-bpf` installs the toolchain from the currently installed solana CLI tools. You may need to upgrade those tools if you encounter any version incompatibilities.

## Deploy your Domichain program

Using the Domichain CLI, you can deploy your program to your currently selected cluster:

```bash
solana program deploy ./target/deploy/hello_world.so
```

Once your Domichain program has been deployed (and the transaction [finalized](../cluster/commitments.md)), the above command will output your program's public address (aka its "program id").

```bash
# example output
Program Id: EFH95fWg49vkFNbAdw9vy75tM7sWZ2hQbTTUmuACGip3
```

#### Congratulations!

You have successfully setup, built, and deployed a Domichain program using the Rust language.

> PS: Check your Domichain wallet's balance again after you deployed. See how much DOMI it cost to deploy your simple program?

## Next steps

See the links below to learn more about writing Rust based Domichain programs:

- [Overview of writing Domichain programs](../developing/on-chain-programs/overview)
- [Learn more about developing Domichain programs with Rust](../developing/on-chain-programs/developing-Rust)
- [Debugging on chain programs](../developing/on-chain-programs/debugging)
