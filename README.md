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

Read more about the phragmen's method [here](https://wiki.polkadot.network/docs/en/learn-phragmen). What this code implements is the sequential method with
two iterations of postprocessing. This is fixed for now, since it is also fixed in srml-staking and
might change over time.

## FAQ

> Is it sorted based on stake?

**NOT AT ALL**. Phragmen's main objective is to maximise the minimum amount at stake, aka. _slot
stake_ which is also outputted per execution.

> Will my validator keep its spot if more slots become available?

**FOR SURE YES**. Phragmen choses the results __in order__. if you ask for 10 elected candidates, and
then 50, the first 10 will be the same in the two runs, given the same input. For example, your validator in spot 21 should always be in spot 21, regardless of if you ask for 30 or 40 elected candidates. 

**Note that** since we do the post-processing, the nominations that your candidate end up with migh differ if more slots become available. Hence, the total stake of your candidate might also differ.
