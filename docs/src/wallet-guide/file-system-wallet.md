---
title: File System Wallet
---

This document describes how to create and use a file system wallet with the
Domichain CLI tools. A file system wallet exists as an unencrypted keypair file
on your computer system's filesystem.

> File system wallets are the **least secure** method of storing DOMI tokens. Storing large amounts of tokens in a file system wallet is **not recommended**.

## Before you Begin

Make sure you have
[installed the Domichain Command Line Tools](../cli/install-domichain-cli-tools.md)

## Generate a File System Wallet Keypair

Use Domichain's command-line tool `domichain-keygen` to generate keypair files. For
example, run the following from a command-line shell:

```bash
mkdir ~/my-domichain-wallet
domichain-keygen new --outfile ~/my-domichain-wallet/my-keypair.json
```

This file contains your **unencrypted** keypair. In fact, even if you specify
a password, that password applies to the recovery seed phrase, not the file. Do
not share this file with others. Anyone with access to this file will have access
to all tokens sent to its public key. Instead, you should share only its public
key. To display its public key, run:

```bash
domichain-keygen pubkey ~/my-domichain-wallet/my-keypair.json
```

It will output a string of characters, such as:

```text
ErRr1caKzK8L8nn4xmEWtimYRiTCAZXjBtVphuZ5vMKy
```

This is the public key corresponding to the keypair in
`~/my-domichain-wallet/my-keypair.json`. The public key of the keypair file is
your _wallet address_.

## Verify your Address against your Keypair file

To verify you hold the private key for a given address, use
`domichain-keygen verify`:

```bash
domichain-keygen verify <PUBKEY> ~/my-domichain-wallet/my-keypair.json
```

where `<PUBKEY>` is replaced with your wallet address.
The command will output "Success" if the given address matches the
one in your keypair file, and "Failed" otherwise.

## Creating Multiple File System Wallet Addresses

You can create as many wallet addresses as you like. Simply re-run the
steps in [Generate a File System Wallet](#generate-a-file-system-wallet-keypair)
and make sure to use a new filename or path with the `--outfile` argument.
Multiple wallet addresses can be useful if you want to transfer tokens between
your own accounts for different purposes.
