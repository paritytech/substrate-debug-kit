# Phrag.. what?

Read more about the phragmen's method [here](https://wiki.polkadot.network/docs/en/learn-phragmen).

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

