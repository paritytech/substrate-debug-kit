# Substrate Runtime Migration Dry Run

## Current Process

> My best-effort at making an end-to-end migration testing tool in the least amount of time.

The setup is as follows:

- You need to have such a directory structure: This repo and substrate and polkadot need to be set as siblings.

```
.
|--/substrate
|--/polkadot
|--/substrate-debug-kit
```

- Needless to say, you need a locally synced node of your desired chain with unsafe RPCs open.

- You always want your substrate to point to either the one being used in polkadot (if you are right before a release), or a branch ahead of master (while developing a new feature that is not merged yet). All in all though, the main point is that the sibling polkadot and substrate need to be compatible.

- You need to make sure that the path of all substrate dependencies is overridden. For polkadot, it is easy: `/migration-dry-run/.cargo/config` that has `path = ["../substrate"]` in it will do. Sadly, this won't cover the case of dependencies directly at `crates.io`. For example, this crate depends on `remote-externalities` from this folder, which depends on `sp-core = "2.0.0"`. The path override will not fix this. Two alternatives exist:
	- use my (almost deprecated) node script to override deps: `node update_cargo.js local`. This will move all the dependencies of the main creates in this repo (including `remote-externalities` and `sub-storage` that we need) to point to the local substrate.
	- Alternatively, in this crate (`./migration-dry-run/Cargo.toml`) there is a `[path.crates.io` that manually overrides all the `sp-*` and `frame-*` dependencies. This should also ensure that all substrate dependencies are pointing to the correct local one. Just make sure that this is not commented, and there's nothing more to do.

By this point, you should have all the 3 repos with a rather singular dependency tree (`polkadot` and everything in `substrate-debug-kit`) will use `substrate`. We already know that the `substrate` and `polkadot` are compatible. There is a small chance that the `sp-*` crates in substrate are not compatible with something in `substrate-debug-kit`. This is quite rare and if so, please make an issue here.

This should be it to make sure everything compiles well. Now let's look at what you can do with the code.

- There's one dependency called `node-runtime`. You can rename this to point to either polkadot or kusama. Ideally, the code should not see any difference as all the types imported from this crate is same between polkadot and kusama.

- You need to change `type Migrations = ` to a tuple of all of the custom migrations within the `node_runtime` (to be found as the last argument passed to `type Executive`). For now we don't have a standard for this, but it would be quite easy to establish one -- see below.

- The code will then scrape the chain (takes a while), run the following migrations:
	- `system`
	- anything in `type Migrations` (can be empty).
	- everything defined in modules.


## What else need be done:

- The type `AllModule` need to be made pubic in substrate's `construct_runtime`.
- Each of our runtimes (polkadot and kusama) need to expose a type (similar to `type Block`): `type CustomMigrations`. This will be imported and used here.
- trait `OnRuntimeUpgrade` could expose another function `fn verify() -> Result<(), _>`. This function **will never be used** inside substrate but will only be exposed for other tools to be able to verify _a particular_ migration. Thus, each pallet that implements `OnRuntimeUpgrade` internally need to implement the `verify` as well (and make sure that they are in sync).  Each custom migration that we feed to `Executive` directly will also implement the same.

> Implementing this additional stuff might be pain in decl_* macros, and quite the charm with frame V2 macros.


## Questions:

- I am not quite sure if an interface as simple as `fn verify() -> Result<(), _>` is enough. One example that I would like to have myself is an ability to compare the pre-migration state with the post-migration state. To achieve this in a simple and ugly fashion: Two functions will be added, `fn pre_migration_checks()` and `fn post_migration_checks()`. Then hypothetically we can write a random sample of the old-state into storage (into some random key -- remember that this will never run on-chain, only in some test env), read it back in the `post-migration` and then check some stuff.

- This is using remote-externalities to dump the whole state into a TestExternalities for usage. I used to be able to do this (rpc call to get the all key values pairs at prefix `0x`) but the last few times I tried it never finishes (waiting an hour). There might be an issue somewhere, but end of the day this shouldn't be a serious issue.
