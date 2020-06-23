# Offline Phragmen

> Last update: Supports both polkadot and kusama using types from
> [v0.8.11](https://github.com/paritytech/polkadot/releases/tag/v0.8.11).


> Note that this software and repo is highly fragile and for now needs some manual work to be kept
> up to date with new releases of the polkadot repo.

> Substrate seminar session about this repo: [youtube.com/watch?v=6omrrY11HEg](youtube.com/watch?v=6omrrY11HEg)

# Usage

A swiss army like tool for various debug and diagnosis operations on top of Polkadot and Kusama. You
need to pass two arguments to specify the chain:

1. `--network` will change the address format and token display name.
2. `--no-default-features --features [polkadot/kusama]` will import the correct runtime. The default
   is currently `polkadot`. This will only affect sub-commands that require the runtime, such as
   those that read events, or need to decode `Call` and `Block`.
3. `--uri` connect the the appropriate node based on the above two.

Current features (most of each being a sub-command) include:

- Staking: Running the staking election algorithm offchain. It directly scrapes the chain and uses
  the same crate as used in substrate, namely `sp-npos-elections`, hence it is the most accurate
  staking election prediction tool. It also supports operations such as `reduce()` and
  `balance_solution()` operations that are done bu validators prior to solution submissions.
- Council: Same as staking, but can run the election for council.
- Dangling Nominators: displays the list of nominators who have voted for recently slashed
  validators.

All of the above stable commands support being passed a `--at` as well to perform the same operation
at a specific block.

And a range of unstable, playground features (see `fn run` in `playground.rs`):

- `dust`: A `dust`-like tool to show the storage usage per module.
- `last_election_submission`: scrapes all of the transactions in recent blocks that submitted
  staking election solutions.
- `account_balance_history`: shows tha balance history of an account.

Run with `--help` for more info.

### Logging

Scripts output additional information as logs. You need to enable them by setting `RUST_LOG`
environment variable.

Also, you can always use `-v`, `-vv`, ... to get more output out of each script.

## Example usage

- Run the council election with 25 members instead of 20.

```
RUST_LOG=offline-phragmen=trace cargo run -- council --count 25
```

- Run the staking election with no equalization at a particular block number

```
cargo run --at 8b7d6e14221b4fefc4b007660c80af6d4a9ac740c50b6e918f61d521553cd17e staking
```

- Run the election with only 50 slots, and print all the nominator distributions

```
cargo run -- -vv staking --count 50
```

- Run the above again now with `reduce()` and see how most nominator edges are... reduced.

```
cargo run -- -vv staking --count 50 --reduce
```

- Run the above again now against a remote node.

```
cargo run -- --uri wss://kusama-rpc.polkadot.io/ -vv staking --count 50 --reduce
```

### Connecting to a node

By default it will attempt to connect to a locally running node running at `ws://127.0.0.1:9944`.

Connect to a different node using the `--uri` argument e.g. `--uri wss://kusama-rpc.polkadot.io/`.

- **`ws://`** prefix: plain (unencrypted) websockets connection.
- **`wss://`** prefix: TLS (encrypted) websockets connection.

# Phrag.. what?

Read more about the phragmen's method [here](https://wiki.polkadot.network/docs/en/learn-phragmen).
What `substrate/core/phragmen` code implements is the sequential method with two iterations of
postprocessing. This is fixed for now, since it is also fixed in srml-staking and might change over
time.

The repository is just an RPC wrapper around the code in substrate.

## FAQ

> Is it sorted based on stake?

**NOT AT ALL**. Phragmen's main objective is to maximise the minimum amount at stake, aka. _slot
stake_ which is also outputted per execution.

> Will my validator keep its spot if more slots become available?

**FOR SURE YES**. Phragmen choses the results __in order__. if you ask for 10 elected candidates,
and then 50, the first 10 will be the same in the two runs, given the same input. For example, your
validator in spot 21 should always be in spot 21, regardless of if you ask for 30 or 40 elected
candidates.

**Note that** since we do the post-processing, the nominations that your candidate end up with migh
differ if more slots become available. Hence, the total stake of your candidate might also differ.
