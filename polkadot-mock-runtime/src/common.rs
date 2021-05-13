// very basic types.
pub type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
pub type BlockNumber = u32;
pub type Moment = u64;
pub type Balance = u128;
pub type Header = sp_runtime::generic::Header<BlockNumber, Hashing>;
pub type Hash = sp_core::H256;
pub type Hashing = sp_runtime::traits::BlakeTwo256;
pub type Index = u32;
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
pub type Signature = sp_runtime::MultiSignature;

// some sp-* types
pub type SessionIndex = u32;

// slightly less simple types.
pub type UncheckedExtrinsicOf<Call, SignedExtra> =
	sp_runtime::generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
pub type BlockOf<UExt> = sp_runtime::generic::Block<Header, UExt>;

pub struct AllowAllCalls;
impl<T> frame_support::traits::Filter<T> for AllowAllCalls {
	fn filter(_: &T) -> bool {
		// we allow all calls in mock runtime
		true
	}
}

pub type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
static_assertions::assert_eq_size!(Balance, u128);

pub const PARACHAIN_KEY_TYPE_ID: sp_core::crypto::KeyTypeId = sp_core::crypto::KeyTypeId(*b"para");
mod validator_app {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, super::PARACHAIN_KEY_TYPE_ID);
}
pub type ValidatorId = validator_app::Public;

// timing constants. same for polkadot and kusama
pub const MILLISECS_PER_BLOCK: Moment = 6000;
pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = 4 * HOURS;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
