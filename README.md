# Offline Phragmen

> Last update: Supports both polkadot and kusama using types from
> [v0.8.8](https://github.com/paritytech/polkadot/releases/tag/v0.8.8).


> Note that this software and repo is highly fragile and for now needs some manual work to be kept
> up to date with new releases of the polkadot repo.

---


Simple script that scrapes the chain and runs the Phragmén election algorithm.

The main aim of this project is to always work with the latest Kusama release, and later on
Polkadot.

Currently, it supports running phragmen on both staking and council election. see the usage details
below for more info.

Notable features for staking:
- Can optionally run `equalize()` a number of times on the solution.
- Can optionally `reduce()` the solution.
- Can run the script on any block number. Default is always the latest finalized.

# How to use

Top level parameters as follows. These can be fed to all scripts. For staking/council dependent
parameters, run the appropriate `--help` command to see the info (`cargo run -- staking --help`).

```
offline-phragmen 1.0
Parity Technologies <admin@parity.io>
Diagnostic lab for everything election and phragmen related in a substrate chain.


OPTIONS:
        --at <at>              scrape the data at the given block hash. Default will be the head of the chain
    -n, --network <network>    network address format. Can be kusama|polkadot|substrate. Default is kusama
    -u, --uri <URI>            websockets uri of the substrate node

SUBCOMMANDS:
    council                Runs the phragmen election for the elections-phragmen module (usually used for council).
    dangling-nominators    Get the dangling nominators in staking. Don't forget to turn on logging.
    help                   Prints this message or the help of the given subcommand(s)
    playground             Runs any program that you put in src/subcommands/playground.rs. No parameters are
                           accepted.
    staking                Runs the phragmen election for staking validators and nominators.
```

## Example usage

- Run the council election with 25 members instead of 20.

```
RUST_LOG=offline-phragmen=trace cargo run -- council --count 25
```

- Run the staking election with no equalization at a particular block number

```
RUST_LOG=offline-phragmen=trace cargo run  --at 8b7d6e14221b4fefc4b007660c80af6d4a9ac740c50b6e918f61d521553cd17e staking
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

### Logging

Scripts output additional information as logs. You need to enable them by setting `RUST_LOG`
environment variable.

Also, you can always use `-v`, `-vv`, ... to get more output out of each script.

### Connecting to a node

By default it will attempt to connect to a locally running node running at `ws://127.0.0.1:9944`.

Connect to a different node using the `--uri` argument e.g. `--uri wss://kusama-rpc.polkadot.io/`.

### Uri Format

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
