# sub-tokens

Small crate to represent tokens in Polkadot, Kusama and Westend. Also provides macro and
functionalities for any substrate based chain.

## Usage:

### Default Tokens: `DOT`, `KSM`, and `WND`.
Current crate provides implementations for DOT, KSM, and WND. note that DOTs are 10 decimal
points and the other two are 12.

```rust
use sub_tokens::DOT;

// 100 new dot, 1 old dot.
let dots = DOT::from(1_000_000_000_000u128);

// provides display and format implementations.
assert_eq!(format!("{}", dots), "100,000 DOT");
assert_eq!(format!("{:?}", dots), "100,000 DOT (1,000,000,000,000)");
```

### Custom tokens

New tokens can be built from the provided macro.

```rust
use sub_tokens::impl_token;

// u32 token with 3 decimal points named KIZ.
impl_token!(KIZ, 1000u32, u32);

let kiz = KIZ::from(100);
assert_eq!(format!("{}", kiz), "0,100 KIZ");
assert_eq!(format!("{:?}", kiz), "0,100 KIZ (100)");
```

### Dynamic Tokens

A dynamic token is also provided that can be used in applications that need to dynamically
decide to which chain to connect. This token type works only with u128.

```rust
// the alias that you will use in your crate.
type MyToken = sub_tokens::dynamic::DynamicToken;

// set the name
sub_tokens::dynamic::set_name("CST");
sub_tokens::dynamic::set_decimal_points(1000);

assert_eq!(format!("{}", MyToken::from(100)), "0,100 CST");
assert_eq!(format!("{:?}", MyToken::from(100)), "0,100 CST (100)");
```
