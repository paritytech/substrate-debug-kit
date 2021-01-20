# Substrate Runtime Migration Dry Run

## Current Process

> My best effort at making an end-to-end migration testing tool.

The setup is as follows:

- You need to have such a directory structure: This repo and substrate and polkadot need to be set as siblings.

```
.
|--/substeate
|--/polkadot
|--/substrate-debug-kit
```

- You always want your substrate to point to either the latest master (if you are right before a release), or a branch ahead of master (while developing a new feature that is not merged yet).

- You need to have a path override (to the sibling substrate folder) for all the substrate dependencies in the current working directory (`./substrate-debug-kit/migration-dry-run/.cargo`). Also, you need to have the same one in your polkadot folder as well (`./polkadot/.cargo` -- I am not quite sure about the logic of these overrides, cargo warns that they are buggy, I just happen to know that this works).

- But there's more: The dependencies from within substrate-debug-kit (i.e. `remote-externalities`) also point to substrate and they need to match the rest of the substrate dependencies. Maybe there's a way to `path` override this as well, but for now I have a script for this. Simply go to the root of `substrate-debug-kit` and run `node update_cargo.js local`.

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
