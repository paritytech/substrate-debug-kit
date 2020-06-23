# How to update this project later versions of polkadot/kusama.

#### Context

For now, we explicitly depend on two problematic crates.

- `node-runtime`: which is aliased most often to the `kusama-runtime`. This is needed because we
  want access to concrete types such as `Event` and `SignedBlock` that are only defined in the
  runtime.

  Note that the `SignedBlock` can actually easily be re-created, but I am not yet sure about
  `Event`. All in all, these types are dynamic based on the runtime version and need to be ideally
  fetched from the metadata, but we don't have a clear way for that.

- `node-primitives`: to give us access to things such `Balance` and `AccountId`. Similar story as
  above.

Moreover, there's also the problem of having access to other runtime types such as variants of the
`Call` enum of a specific

### Temporary solution

Hence, our temporary solution is to import these crates from exact version of the polkadot repo. How
we do this is:

- For each polkadot release that we want to support, we create branch from that specific commit of
  polkadot, with a name such as `offline-phragmen-v0.8.xxx`.
- In this branch we apply two important changes:
  1. We pin all the substrate dependencies to whatever is mentioned in the `Cargo.lock`. This is
     needed otherwise the `node-runtime` that we import here will be built against substrate master.
  2. Apply the small changes such as re-exporting `system` and `staking`. For this, you can usually
     look at the previous branch of `offline-phragmen-v0.8.xxx` and cherry pick the commit.

  to facilitate, and keep this clear, always do these in two separate commits, as such:
  	https://github.com/paritytech/polkadot/commits/offline-phragmen-v0.8.11

- Do a quick `cargo check` or something, and push this branch.

- Finally, in this repo, update all substrate dependencies in to the same one pinned above, and bump
  the `offline-phragmen-v0.8.xxx` as well to the newly created one.

