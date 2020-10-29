#[doc(hidden)]
pub use separator::Separatable;
#[doc(hidden)]
pub use std::{cell::RefCell, convert::TryInto, fmt};

macro_rules! impl_token {
	($name:ident, $decimals:expr, $type:ty) => {
		pub struct $name($type);

		impl $name {
			pub fn from(t: $type) -> Self {
				Self(t)
			}
		}

		impl std::fmt::Display for $name {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(
					f,
					"{},{:0>3} {}",
					(self.0 / $decimals).separated_string(),
					self.0 % $decimals / ($decimals / 1000),
					stringify!($name)
				)
			}
		}

		impl std::fmt::Debug for $name {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(
					f,
					"{},{:0>3}{} ({})",
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
