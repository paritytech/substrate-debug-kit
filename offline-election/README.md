# Offline elections

Run election algorithms of substrate (all under `sp-npos-elections`) offline.

> Substrate seminar about offchain phragmen and how the staking pallet works in substrate. [https://www.youtube.com/watch?v=MjOvVhc1oXw](https://www.youtube.com/watch?v=MjOvVhc1oXw).

> Substrate seminar session about this repo prior to the overhaul (`offline-phragmen`):
> [youtube.com/watch?v=6omrrY11HEg](youtube.com/watch?v=6omrrY11HEg)


### Builders

Several tools have already built on top of this repo, such as polkadot.por and TODO.

Note that the npos results generate by this repo will not be exactly equal to that of polkadot and
kusama. This is highly dependent on the arguments passed to the `staking` sub-command. See
[`PHRAGMEN.md`]() for more detail.

## Usage

Simply run `--help`.

```
Offline elections app.

Provides utilities and debug tools around the election pallets of a substrate chain offline.

Can be used to predict next elections, diagnose previous ones, and perform checks on validators and nominators.

USAGE:
    offline-election [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help
            Prints help information

    -V, --version
            Prints version information

    -v
            Print more output


OPTIONS:
        --at <at>
            The block number at which the scrap should happen. Use only the hex value, no need for a `0x` prefix

    -n, --network <network>
            Network address format. Can be kusama|polkadot|substrate.

            This will also change the token display name. [default: polkadot]
        --uri <uri>
            The node to connect to [default: ws://localhost:9944]


SUBCOMMANDS:
    command-center         Display the command center of the staking panel
    council                Run the council election
    current                Display the current validators
    dangling-nominators    Show the nominators who are dangling:
    help                   Prints this message or the help of the given subcommand(s)
    next                   Display the next queued validators
    nominator-check        The general checkup of a nominator
    staking                Run the staking election
    validator-check        The general checkup of a validators
```
## Install

TODO:

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

## Connecting to a node

By default it will attempt to connect to a locally running node running at `ws://127.0.0.1:9944`.

Connect to a different node using the `--uri` argument e.g. `--uri wss://kusama-rpc.polkadot.io/`.

- **`ws://`** prefix: plain (unencrypted) websockets connection.
- **`wss://`** prefix: TLS (encrypted) websockets connection.

## Logging

Scripts output additional information as logs. You need to enable them by setting `RUST_LOG`
environment variable.

Also, you can always use `-v`, `-vv`, ... to get more output out of each script.
