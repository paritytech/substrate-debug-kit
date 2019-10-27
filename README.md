# Offline Phragmen

Simple script that runs the phragmen method on the current validator candidates of a locally running
substrate chain.

```bash
offline-phragmen 0.1
Kian Paimani <kian@parity.io>
Runs the phragmen election algorithm of any substrate chain with staking module offline (aka. off the chain) and
predicts the results.

USAGE:
    offline-phragmen [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --count <VALIDATOR_COUNT>            count of validators to elect. Should be equal to
                                             chain.staking.validatorCount. Default is 50.
    -m, --min-count <MIN_VALIDATOR_COUNT>    minimum number of validators to elect. If less candidates are available,
                                             phragmen will go south. Should be equal to
                                             chain.staking.minimumValidatorCount. Default is 10.
    -n, --network <NETWORK>                  network address format. Can be kusama|polkadot|substrate. Default is
                                             kusama.
    -o, --output <FILE>                      Json output file name. dumps the results into if given.
```

# Phrag.. what?

Read more about the phragmen's method here. What this code implements is the sequential method with
two iterations of postprocessing. This is fixed for now, since it is also fixed in srml-staking and
might change over time.

## FAQ

> Is it sorted based on stake?

**NOT AT ALL**. Phragmen's main objective is to maximise the minimum amount at stake, aka. _slot
stake_ which is also outputted per execution.

> Will my validator keep its spot if more slots are available

**FOR SURE YES**. Phragmen choses the results in order. if you ask for 10 elected candidates, and
then 50, the first 10 will be the same in the two runs, given the same input.
