//! Small crate to represent tokens in Polkadot, Kusama and Westend. Also provides macro and
//! functionalities for any substrate based chain.
//!
//! # Usage:
//!
//! ## Default Tokens: `DOT`, `KSM`, and `WND`.
//! Current crate provides implementations for DOT, KSM, and WND. note that DOTs are 10 decimal
//! points and the other two are 12.
//!
//! ```
//! use sub_tokens::DOT;
//!
//! // 100 new dot, 1 old dot.
//! let dots = DOT::from(1_000_000_000_000u128);
//!
//! // provides display and format implementations.
//! assert_eq!(format!("{}", dots), "100,000 DOT");
//! assert_eq!(format!("{:?}", dots), "100,000 DOT (1,000,000,000,000)");
//! ```
//!
//! ## Custom tokens
//!
//! New tokens can be built from the provided macro.
//!
//! ```
//! use sub_tokens::impl_token;
//!
//! // u32 token with 3 decimal points named KIZ.
//! impl_token!(KIZ, 1000u32, u32);
//!
//! let kiz = KIZ::from(100);
//! assert_eq!(format!("{}", kiz), "0,100 KIZ");
//! assert_eq!(format!("{:?}", kiz), "0,100 KIZ (100)");
//! ```
//!
//! ## Dynamic Tokens
//!
//! A dynamic token is also provided that can be used in applications that need to dynamically
//! decide to which chain to connect. This token type works only with u128.
//!
//! ```
//! // the alias that you will use in your crate.
//! type MyToken = sub_tokens::dynamic::DynamicToken;
//!
//! // set the name
//! sub_tokens::dynamic::set_name("CST");
//! sub_tokens::dynamic::set_decimal_points(1000);
//!
//! assert_eq!(format!("{}", MyToken::from(100)), "0,100 CST");
//! assert_eq!(format!("{:?}", MyToken::from(100)), "0,100 CST (100)");
//! ```

#[doc(hidden)]
pub use separator::Separatable;
#[doc(hidden)]
pub use std::{cell::RefCell, convert::TryInto, fmt};

#[macro_export]
macro_rules! impl_token {
	($name:ident, $decimals:expr, $type:ty) => {
		pub struct $name($type);

		impl $name {
			pub fn from(t: $type) -> Self {
				Self(t)
			}
		}

		impl $crate::fmt::Display for $name {
			fn fmt(&self, f: &mut $crate::fmt::Formatter) -> $crate::fmt::Result {
				use $crate::Separatable;
				write!(
					f,
					"{},{:0>3} {}",
					(self.0 / $decimals).separated_string(),
					self.0 % $decimals / ($decimals / 1000),
					stringify!($name)
				)
			}
		}

		impl $crate::fmt::Debug for $name {
			fn fmt(&self, f: &mut $crate::fmt::Formatter) -> $crate::fmt::Result {
				use $crate::Separatable;
				write!(
					f,
					"{},{:0>3} {} ({})",
					self.0 / $decimals,
					self.0 % $decimals / ($decimals / 1000),
					stringify!($name),
					self.0.separated_string(),
				)
			}
		}
	};
}

impl_token!(DOT, 1_0_000_000_000u128, u128);
impl_token!(WND, 1_000_000_000_000u128, u128);
impl_token!(KSM, 1_000_000_000_000u128, u128);

pub mod dynamic {
	use super::*;
	use std::{cell::RefCell, fmt};

	thread_local! {
		/// Decimal points of the currency based on the network.
		static DECIMAL_POINTS: RefCell<u128> = RefCell::new(1_000_000_000_000u128);

		/// Name of the currency token based on the network.
		static TOKEN_NAME: RefCell<&'static str> = RefCell::new("GTK");
	}

	pub fn set_name(name: &'static str) {
		TOKEN_NAME.with(|v| *v.borrow_mut() = name);
	}

	pub fn set_decimal_points(decimal: u128) {
		DECIMAL_POINTS.with(|v| *v.borrow_mut() = decimal);
	}

	/// Wrapper to pretty-print currency token.
	pub struct DynamicToken(u128);

	impl DynamicToken {
		pub fn from(x: u128) -> Self {
			Self(x)
		}
	}

	impl std::fmt::Debug for DynamicToken {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			let num: u128 = self.0.try_into().unwrap();
			let decimal = DECIMAL_POINTS.with(|v| *v.borrow());
			let name = TOKEN_NAME.with(|v| *v.borrow());
			write!(
				f,
				"{},{:0>3} {} ({})",
				self.0 / decimal,
				self.0 % decimal / (decimal / 1000),
				name,
				num.separated_string()
			)
		}
	}

	impl std::fmt::Display for DynamicToken {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			let num: u128 = self.0.try_into().unwrap();
			let decimal = DECIMAL_POINTS.with(|v| *v.borrow());
			let name = TOKEN_NAME.with(|v| *v.borrow());
			write!(
				f,
				"{},{:0>3} {}",
				(num / decimal).separated_string(),
				num % decimal / (decimal / 1000),
				name
			)
		}
	}
}
