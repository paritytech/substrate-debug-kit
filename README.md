# Substrate debug-kit 🛠⚙️

A collection of debug tools and libraries around substrate chains.

> This project has evolved from the historical name **`offline-phragmen`**. I first created this repo
> prior to [Kusama](https://kusama.network/)'s NPoS enabling as a tool to predict the outcome. Henceforth, it has evolved
> into this repo. This functionality is still provided in the [`offline-elections`](https://github.com/paritytech/offline-phragmen/tree/master/offline-election) crate.

## Overview

- **`sub-storage`**: This is the backbone of all of the crates in this repo. It provides a minimal
  wrapper around substrate's storage rpc call for easier use. It provides all you need to read any
  module's storage items, constants, and metadata. All of this is independent of any chain or pallet
  and should work in any substrate chain. Additionally, it provide some pallet-dependent helpers as
  well under the `helpers` feature (such as reading identity of an account).
- **`sub-du`**: a [**d**isk-**u**sage](https://en.wikipedia.org/wiki/Du_(Unix))-like tool that prints the storage usage of a chain. It reads all the info
  it needs from metadata, so independent chain or runtime. Arguably not super useful, but I find it
  cool.
- **`offline-elections`**: The historical main purpose of this repo. It can scrape the staking
  module's info and run election algorithms of `sp-npos-elections` offline. **Given the correct
  parameters**, it can be used to predict the next validator set. It also provide other election
  related functionalities. See the sub-commands for more info.
- **`remote-externalities`**: It provides the ability to write simple rust unit tests over a
  specific state of a chain. It can be very useful to debug breaking changes and storage migrations.
- **`laboratory`**: This is where I try new stuff.
