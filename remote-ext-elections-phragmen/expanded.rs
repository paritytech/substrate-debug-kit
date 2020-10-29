#![feature(prelude_import)]
//! A remote ext template for elections-phragmen pallet in a live chain.
//!
//! Note: You need to have this repo cloned next to a local substrate repo, and have all the
//! dependencies pointed to a local one. To do so, use `node update_cargo.js local`.
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
use frame_support::{
    impl_outer_origin, migration::*, parameter_types, traits::Get,
    weights::constants::RocksDbWeight, IterableStorageMap, StorageMap, StoragePrefixedMap,
    Twox64Concat,
};
use pallet_elections_phragmen::*;
use paste::paste;
use sp_core::H256;
use sp_runtime::traits::IdentityLookup;
use std::cell::RefCell;
use sub_storage::Hash;
pub mod time {
    use super::*;
    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 1 * HOURS;
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}
pub struct CurrencyToVoteHandler;
impl CurrencyToVoteHandler {
    fn factor() -> Balance {
        (Balances::total_issuance() / u64::max_value() as Balance).max(1)
    }
}
impl sp_runtime::traits::Convert<Balance, u64> for CurrencyToVoteHandler {
    fn convert(x: Balance) -> u64 {
        (x / Self::factor()) as u64
    }
}
impl sp_runtime::traits::Convert<u128, Balance> for CurrencyToVoteHandler {
    fn convert(x: u128) -> Balance {
        x * Self::factor()
    }
}
pub(crate) type AccountId = sp_core::crypto::AccountId32;
pub(crate) type BlockNumber = u32;
pub(crate) type Balance = u128;
pub(crate) type Moment = u64;
pub(crate) type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;
pub(crate) type Balances = pallet_balances::Module<Runtime>;
pub(crate) type System = frame_system::Module<Runtime>;
pub(crate) type Elections = pallet_elections_phragmen::Module<Runtime>;
pub(crate) type WrongElections = pallet_elections_phragmen_faulty::Module<Runtime>;
pub struct Runtime;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Runtime {
    #[inline]
    fn clone(&self) -> Runtime {
        match *self {
            Runtime => Runtime,
        }
    }
}
impl ::core::marker::StructuralEq for Runtime {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Runtime {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for Runtime {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Runtime {
    #[inline]
    fn eq(&self, other: &Runtime) -> bool {
        match *other {
            Runtime => match *self {
                Runtime => true,
            },
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Runtime => {
                let mut debug_trait_builder = f.debug_tuple("Runtime");
                debug_trait_builder.finish()
            }
        }
    }
}
pub struct Origin {
    caller: OriginCaller,
    filter: ::frame_support::sp_std::rc::Rc<
        Box<dyn Fn(&<Runtime as frame_system::Trait>::Call) -> bool>,
    >,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Origin {
    #[inline]
    fn clone(&self) -> Origin {
        match *self {
            Origin {
                caller: ref __self_0_0,
                filter: ref __self_0_1,
            } => Origin {
                caller: ::core::clone::Clone::clone(&(*__self_0_0)),
                filter: ::core::clone::Clone::clone(&(*__self_0_1)),
            },
        }
    }
}
#[cfg(not(feature = "std"))]
impl ::frame_support::sp_std::fmt::Debug for Origin {
    fn fmt(
        &self,
        fmt: &mut ::frame_support::sp_std::fmt::Formatter,
    ) -> ::frame_support::sp_std::result::Result<(), ::frame_support::sp_std::fmt::Error> {
        fmt.write_str("<wasm:stripped>")
    }
}
impl ::frame_support::traits::OriginTrait for Origin {
    type Call = <Runtime as frame_system::Trait>::Call;
    type PalletsOrigin = OriginCaller;
    type AccountId = <Runtime as frame_system::Trait>::AccountId;
    fn add_filter(&mut self, filter: impl Fn(&Self::Call) -> bool + 'static) {
        let f = self.filter.clone();
        self.filter =
            ::frame_support::sp_std::rc::Rc::new(Box::new(move |call| f(call) && filter(call)));
    }
    fn reset_filter(&mut self) {
        let filter =
            <<Runtime as frame_system::Trait>::BaseCallFilter as ::frame_support::traits::Filter<
                <Runtime as frame_system::Trait>::Call,
            >>::filter;
        self.filter = ::frame_support::sp_std::rc::Rc::new(Box::new(filter));
    }
    fn set_caller_from(&mut self, other: impl Into<Self>) {
        self.caller = other.into().caller
    }
    fn filter_call(&self, call: &Self::Call) -> bool {
        (self.filter)(call)
    }
    fn caller(&self) -> &Self::PalletsOrigin {
        &self.caller
    }
    /// Create with system none origin and `frame-system::Trait::BaseCallFilter`.
    fn none() -> Self {
        frame_system::RawOrigin::None.into()
    }
    /// Create with system root origin and no filter.
    fn root() -> Self {
        frame_system::RawOrigin::Root.into()
    }
    /// Create with system signed origin and `frame-system::Trait::BaseCallFilter`.
    fn signed(by: <Runtime as frame_system::Trait>::AccountId) -> Self {
        frame_system::RawOrigin::Signed(by).into()
    }
}
#[allow(non_camel_case_types)]
pub enum OriginCaller {
    system(frame_system::Origin<Runtime>),
    #[allow(dead_code)]
    Void(::frame_support::Void),
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::core::clone::Clone for OriginCaller {
    #[inline]
    fn clone(&self) -> OriginCaller {
        match (&*self,) {
            (&OriginCaller::system(ref __self_0),) => {
                OriginCaller::system(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&OriginCaller::Void(ref __self_0),) => {
                OriginCaller::Void(::core::clone::Clone::clone(&(*__self_0)))
            }
        }
    }
}
#[allow(non_camel_case_types)]
impl ::core::marker::StructuralPartialEq for OriginCaller {}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::core::cmp::PartialEq for OriginCaller {
    #[inline]
    fn eq(&self, other: &OriginCaller) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) };
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) };
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&OriginCaller::system(ref __self_0), &OriginCaller::system(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&OriginCaller::Void(ref __self_0), &OriginCaller::Void(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() },
                }
            } else {
                false
            }
        }
    }
    #[inline]
    fn ne(&self, other: &OriginCaller) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) };
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) };
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&OriginCaller::system(ref __self_0), &OriginCaller::system(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&OriginCaller::Void(ref __self_0), &OriginCaller::Void(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() },
                }
            } else {
                true
            }
        }
    }
}
#[allow(non_camel_case_types)]
impl ::core::marker::StructuralEq for OriginCaller {}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::core::cmp::Eq for OriginCaller {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<frame_system::Origin<Runtime>>;
            let _: ::core::cmp::AssertParamIsEq<::frame_support::Void>;
        }
    }
}
impl core::fmt::Debug for OriginCaller {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::system(ref a0) => fmt.debug_tuple("OriginCaller::system").field(a0).finish(),
            Self::Void(ref a0) => fmt.debug_tuple("OriginCaller::Void").field(a0).finish(),
            _ => Ok(()),
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Encode for OriginCaller {
        fn encode_to<__CodecOutputEdqy: _parity_scale_codec::Output>(
            &self,
            __codec_dest_edqy: &mut __CodecOutputEdqy,
        ) {
            match *self {
                OriginCaller::system(ref aa) => {
                    __codec_dest_edqy.push_byte(0usize as u8);
                    __codec_dest_edqy.push(aa);
                }
                OriginCaller::Void(ref aa) => {
                    __codec_dest_edqy.push_byte(1usize as u8);
                    __codec_dest_edqy.push(aa);
                }
                _ => (),
            }
        }
    }
    impl _parity_scale_codec::EncodeLike for OriginCaller {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Decode for OriginCaller {
        fn decode<__CodecInputEdqy: _parity_scale_codec::Input>(
            __codec_input_edqy: &mut __CodecInputEdqy,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            match __codec_input_edqy.read_byte()? {
                __codec_x_edqy if __codec_x_edqy == 0usize as u8 => Ok(OriginCaller::system({
                    let __codec_res_edqy = _parity_scale_codec::Decode::decode(__codec_input_edqy);
                    match __codec_res_edqy {
                        Err(_) => {
                            return Err("Error decoding field OriginCaller :: system.0".into())
                        }
                        Ok(__codec_res_edqy) => __codec_res_edqy,
                    }
                })),
                __codec_x_edqy if __codec_x_edqy == 1usize as u8 => Ok(OriginCaller::Void({
                    let __codec_res_edqy = _parity_scale_codec::Decode::decode(__codec_input_edqy);
                    match __codec_res_edqy {
                        Err(_) => return Err("Error decoding field OriginCaller :: Void.0".into()),
                        Ok(__codec_res_edqy) => __codec_res_edqy,
                    }
                })),
                _ => Err("No such variant in enum OriginCaller".into()),
            }
        }
    }
};
#[allow(dead_code)]
impl Origin {
    /// Create with system none origin and `frame-system::Trait::BaseCallFilter`.
    pub fn none() -> Self {
        <Origin as ::frame_support::traits::OriginTrait>::none()
    }
    /// Create with system root origin and no filter.
    pub fn root() -> Self {
        <Origin as ::frame_support::traits::OriginTrait>::root()
    }
    /// Create with system signed origin and `frame-system::Trait::BaseCallFilter`.
    pub fn signed(by: <Runtime as frame_system::Trait>::AccountId) -> Self {
        <Origin as ::frame_support::traits::OriginTrait>::signed(by)
    }
}
impl From<frame_system::Origin<Runtime>> for OriginCaller {
    fn from(x: frame_system::Origin<Runtime>) -> Self {
        OriginCaller::system(x)
    }
}
impl From<frame_system::Origin<Runtime>> for Origin {
    /// Convert to runtime origin:
    /// * root origin is built with no filter
    /// * others use `frame-system::Trait::BaseCallFilter`
    fn from(x: frame_system::Origin<Runtime>) -> Self {
        let o: OriginCaller = x.into();
        o.into()
    }
}
impl From<OriginCaller> for Origin {
    fn from(x: OriginCaller) -> Self {
        let mut o = Origin {
            caller: x,
            filter: ::frame_support::sp_std::rc::Rc::new(Box::new(|_| true)),
        };
        if !match o.caller {
            OriginCaller::system(frame_system::Origin::<Runtime>::Root) => true,
            _ => false,
        } {
            ::frame_support::traits::OriginTrait::reset_filter(&mut o);
        }
        o
    }
}
impl Into<::frame_support::sp_std::result::Result<frame_system::Origin<Runtime>, Origin>>
    for Origin
{
    /// NOTE: converting to pallet origin loses the origin filter information.
    fn into(self) -> ::frame_support::sp_std::result::Result<frame_system::Origin<Runtime>, Self> {
        if let OriginCaller::system(l) = self.caller {
            Ok(l)
        } else {
            Err(self)
        }
    }
}
impl From<Option<<Runtime as frame_system::Trait>::AccountId>> for Origin {
    /// Convert to runtime origin with caller being system signed or none and use filter
    /// `frame-system::Trait::BaseCallFilter`.
    fn from(x: Option<<Runtime as frame_system::Trait>::AccountId>) -> Self {
        <frame_system::Origin<Runtime>>::from(x).into()
    }
}
impl frame_system::Trait for Runtime {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Index = u32;
    type BlockNumber = BlockNumber;
    type Call = ();
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = ();
    type MaximumBlockWeight = ();
    type DbWeight = RocksDbWeight;
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = ();
    type AvailableBlockRatio = ();
    type MaximumBlockLength = ();
    type Version = ();
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}
impl pallet_balances::Trait for Runtime {
    type Balance = Balance;
    type MaxLocks = ();
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ();
    type AccountStore = frame_system::Module<Runtime>;
    type WeightInfo = ();
}
impl pallet_timestamp::Trait for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ();
    type WeightInfo = ();
}
const DESIRED_MEMBERS: ::std::thread::LocalKey<RefCell<u32>> = {
    #[inline]
    fn __init() -> RefCell<u32> {
        RefCell::new(13)
    }
    unsafe fn __getit() -> ::std::option::Option<&'static RefCell<u32>> {
        #[thread_local]
        #[cfg(all(
            target_thread_local,
            not(all(target_arch = "wasm32", not(target_feature = "atomics"))),
        ))]
        static __KEY: ::std::thread::__FastLocalKeyInner<RefCell<u32>> =
            ::std::thread::__FastLocalKeyInner::new();
        #[allow(unused_unsafe)]
        unsafe {
            __KEY.get(__init)
        }
    }
    unsafe { ::std::thread::LocalKey::new(__getit) }
};
const DESIRED_RUNNERS_UP: ::std::thread::LocalKey<RefCell<u32>> = {
    #[inline]
    fn __init() -> RefCell<u32> {
        RefCell::new(20)
    }
    unsafe fn __getit() -> ::std::option::Option<&'static RefCell<u32>> {
        #[thread_local]
        #[cfg(all(
            target_thread_local,
            not(all(target_arch = "wasm32", not(target_feature = "atomics"))),
        ))]
        static __KEY: ::std::thread::__FastLocalKeyInner<RefCell<u32>> =
            ::std::thread::__FastLocalKeyInner::new();
        #[allow(unused_unsafe)]
        unsafe {
            __KEY.get(__init)
        }
    }
    unsafe { ::std::thread::LocalKey::new(__getit) }
};
pub struct DesiredMembers;
impl Get<u32> for DesiredMembers {
    fn get() -> u32 {
        DESIRED_MEMBERS.with(|v| v.borrow().clone())
    }
}
impl DesiredMembers {
    #[allow(dead_code)]
    fn set(t: u32) {
        DESIRED_MEMBERS.with(|v| *v.borrow_mut() = t);
    }
}
pub struct DesiredRunnersUp;
impl Get<u32> for DesiredRunnersUp {
    fn get() -> u32 {
        DESIRED_RUNNERS_UP.with(|v| v.borrow().clone())
    }
}
impl DesiredRunnersUp {
    #[allow(dead_code)]
    fn set(t: u32) {
        DESIRED_RUNNERS_UP.with(|v| *v.borrow_mut() = t);
    }
}
pub struct ElectionsPhragmenModuleId;
impl ElectionsPhragmenModuleId {
    /// Returns the value of this parameter type.
    pub const fn get() -> frame_support::traits::LockIdentifier {
        *b"phrelect"
    }
}
impl<I: From<frame_support::traits::LockIdentifier>> ::frame_support::traits::Get<I>
    for ElectionsPhragmenModuleId
{
    fn get() -> I {
        I::from(*b"phrelect")
    }
}
pub struct TermDuration;
impl TermDuration {
    /// Returns the value of this parameter type.
    pub const fn get() -> BlockNumber {
        7 * time::DAYS
    }
}
impl<I: From<BlockNumber>> ::frame_support::traits::Get<I> for TermDuration {
    fn get() -> I {
        I::from(7 * time::DAYS)
    }
}
pub struct CandidacyBond;
impl CandidacyBond {
    /// Returns the value of this parameter type.
    pub const fn get() -> Balance {
        100 * DOLLARS
    }
}
impl<I: From<Balance>> ::frame_support::traits::Get<I> for CandidacyBond {
    fn get() -> I {
        I::from(100 * DOLLARS)
    }
}
pub struct VotingBond;
impl VotingBond {
    /// Returns the value of this parameter type.
    pub const fn get() -> Balance {
        5 * DOLLARS
    }
}
impl<I: From<Balance>> ::frame_support::traits::Get<I> for VotingBond {
    fn get() -> I {
        I::from(5 * DOLLARS)
    }
}
impl pallet_elections_phragmen::Trait for Runtime {
    type ModuleId = ElectionsPhragmenModuleId;
    type Event = ();
    type Currency = Balances;
    type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
    type ChangeMembers = ();
    type InitializeMembers = ();
    type CandidacyBond = CandidacyBond;
    type VotingBond = VotingBond;
    type TermDuration = TermDuration;
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type LoserCandidate = ();
    type KickedMember = ();
    type BadReport = ();
    type WeightInfo = ();
}
impl pallet_elections_phragmen_faulty::Trait for Runtime {
    type ModuleId = ElectionsPhragmenModuleId;
    type Event = ();
    type Currency = Balances;
    type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
    type ChangeMembers = ();
    type InitializeMembers = ();
    type CandidacyBond = CandidacyBond;
    type VotingBond = VotingBond;
    type TermDuration = TermDuration;
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type LoserCandidate = ();
    type KickedMember = ();
    type BadReport = ();
    type WeightInfo = ();
}
const URI: &'static str = "ws://localhost:9944";
pub const DOTS: Balance = 1_000_000_000_000;
pub const DOLLARS: Balance = DOTS / 100;
pub const CENTS: Balance = DOLLARS / 100;
pub const MILLICENTS: Balance = CENTS / 1_000;
pub struct OldAccountInfo<Index, AccountData> {
    /// The number of transactions this account has sent.
    pub nonce: Index,
    /// The number of other modules that currently depend on this account's existence. The account
    /// cannot be reaped until this is zero.
    pub refcount: u8,
    /// The additional data that belongs to this account. Used to store the balance(s) in a lot of
    /// chains.
    pub data: AccountData,
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<Index, AccountData> _parity_scale_codec::Encode for OldAccountInfo<Index, AccountData>
    where
        Index: _parity_scale_codec::Encode,
        Index: _parity_scale_codec::Encode,
        AccountData: _parity_scale_codec::Encode,
        AccountData: _parity_scale_codec::Encode,
    {
        fn encode_to<__CodecOutputEdqy: _parity_scale_codec::Output>(
            &self,
            __codec_dest_edqy: &mut __CodecOutputEdqy,
        ) {
            __codec_dest_edqy.push(&self.nonce);
            __codec_dest_edqy.push(&self.refcount);
            __codec_dest_edqy.push(&self.data);
        }
    }
    impl<Index, AccountData> _parity_scale_codec::EncodeLike for OldAccountInfo<Index, AccountData>
    where
        Index: _parity_scale_codec::Encode,
        Index: _parity_scale_codec::Encode,
        AccountData: _parity_scale_codec::Encode,
        AccountData: _parity_scale_codec::Encode,
    {
    }
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<Index, AccountData> _parity_scale_codec::Decode for OldAccountInfo<Index, AccountData>
    where
        Index: _parity_scale_codec::Decode,
        Index: _parity_scale_codec::Decode,
        AccountData: _parity_scale_codec::Decode,
        AccountData: _parity_scale_codec::Decode,
    {
        fn decode<__CodecInputEdqy: _parity_scale_codec::Input>(
            __codec_input_edqy: &mut __CodecInputEdqy,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            Ok(OldAccountInfo {
                nonce: {
                    let __codec_res_edqy = _parity_scale_codec::Decode::decode(__codec_input_edqy);
                    match __codec_res_edqy {
                        Err(_) => return Err("Error decoding field OldAccountInfo.nonce".into()),
                        Ok(__codec_res_edqy) => __codec_res_edqy,
                    }
                },
                refcount: {
                    let __codec_res_edqy = _parity_scale_codec::Decode::decode(__codec_input_edqy);
                    match __codec_res_edqy {
                        Err(_) => return Err("Error decoding field OldAccountInfo.refcount".into()),
                        Ok(__codec_res_edqy) => __codec_res_edqy,
                    }
                },
                data: {
                    let __codec_res_edqy = _parity_scale_codec::Decode::decode(__codec_input_edqy);
                    match __codec_res_edqy {
                        Err(_) => return Err("Error decoding field OldAccountInfo.data".into()),
                        Ok(__codec_res_edqy) => __codec_res_edqy,
                    }
                },
            })
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl<Index: ::core::fmt::Debug, AccountData: ::core::fmt::Debug> ::core::fmt::Debug
    for OldAccountInfo<Index, AccountData>
{
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            OldAccountInfo {
                nonce: ref __self_0_0,
                refcount: ref __self_0_1,
                data: ref __self_0_2,
            } => {
                let mut debug_trait_builder = f.debug_struct("OldAccountInfo");
                let _ = debug_trait_builder.field("nonce", &&(*__self_0_0));
                let _ = debug_trait_builder.field("refcount", &&(*__self_0_1));
                let _ = debug_trait_builder.field("data", &&(*__self_0_2));
                debug_trait_builder.finish()
            }
        }
    }
}
fn main() -> () {
    async fn main() -> () {
        {
            env_logger::init();
            sp_core::crypto::set_default_ss58_version(
                sp_core::crypto::Ss58AddressFormat::PolkadotAccount,
            );
            let client = sub_storage::create_ws_client(URI).await;
            let elections = <[_]>::into_vec(box [
                "5b81b31f41e4d8372f7b7a3517592a561a18de42debad69e15a3dbdf62ee8cad",
            ])
            .into_iter()
            .rev()
            .map(|h| hex::decode(h).unwrap())
            .map(|v| {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(v.as_ref());
                hash
            })
            .map(Into::into)
            .collect::<Vec<Hash>>();
            use std::collections::BTreeMap;
            let mut slash_record: BTreeMap<AccountId, Vec<(Hash, Balance, Balance)>> =
                BTreeMap::new();
            for at in elections {
                let header = sub_storage::get_header(&client, at).await;
                let parent = header.parent_hash;
                let desired_members = sub_storage::get_const::<u32>(
                    &client,
                    "PhragmenElection",
                    "DesiredMembers",
                    at,
                )
                .await
                .unwrap();
                let desired_runners_up = sub_storage::get_const::<u32>(
                    &client,
                    "PhragmenElection",
                    "DesiredRunnersUp",
                    at,
                )
                .await
                .unwrap();
                (
                    match desired_members {
                        tmp => {
                            {
                                ::std::io::_eprint(::core::fmt::Arguments::new_v1_formatted(
                                    &["[", ":", "] ", " = ", "\n"],
                                    &match (&"src/main.rs", &310u32, &"desired_members", &&tmp) {
                                        (arg0, arg1, arg2, arg3) => [
                                            ::core::fmt::ArgumentV1::new(
                                                arg0,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg1,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg2,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg3,
                                                ::core::fmt::Debug::fmt,
                                            ),
                                        ],
                                    },
                                    &[
                                        ::core::fmt::rt::v1::Argument {
                                            position: 0usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 1usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 2usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 3usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 4u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                    ],
                                ));
                            };
                            tmp
                        }
                    },
                    match desired_runners_up {
                        tmp => {
                            {
                                ::std::io::_eprint(::core::fmt::Arguments::new_v1_formatted(
                                    &["[", ":", "] ", " = ", "\n"],
                                    &match (&"src/main.rs", &310u32, &"desired_runners_up", &&tmp) {
                                        (arg0, arg1, arg2, arg3) => [
                                            ::core::fmt::ArgumentV1::new(
                                                arg0,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg1,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg2,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg3,
                                                ::core::fmt::Debug::fmt,
                                            ),
                                        ],
                                    },
                                    &[
                                        ::core::fmt::rt::v1::Argument {
                                            position: 0usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 1usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 2usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 3usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 4u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                    ],
                                ));
                            };
                            tmp
                        }
                    },
                );
                DesiredMembers::set(desired_members);
                DesiredRunnersUp::set(desired_runners_up);
                remote_externalities :: Builder :: new ( ) . module ( "PhragmenElection" ) . module ( "Balances" ) . module ( "System" ) . module ( "Timestamp" ) . at ( at ) . build_async ( ) . await . execute_with ( | | { let pre_chain_members = Elections :: members ( ) ; let pre_chain_runners_up = Elections :: runners_up ( ) ; let correct_slashed = Elections :: do_phragmen ( ) ; let wrong_slashed = WrongElections :: do_phragmen ( ) ; match & correct_slashed { tmp => { { :: std :: io :: _eprint ( :: core :: fmt :: Arguments :: new_v1_formatted ( & [ "[" , ":" , "] " , " = " , "\n" ] , & match ( & "src/main.rs" , & 330u32 , & "&correct_slashed" , & & tmp ) { ( arg0 , arg1 , arg2 , arg3 ) => [ :: core :: fmt :: ArgumentV1 :: new ( arg0 , :: core :: fmt :: Display :: fmt ) , :: core :: fmt :: ArgumentV1 :: new ( arg1 , :: core :: fmt :: Display :: fmt ) , :: core :: fmt :: ArgumentV1 :: new ( arg2 , :: core :: fmt :: Display :: fmt ) , :: core :: fmt :: ArgumentV1 :: new ( arg3 , :: core :: fmt :: Debug :: fmt ) ] , } , & [ :: core :: fmt :: rt :: v1 :: Argument { position : 0usize , format : :: core :: fmt :: rt :: v1 :: FormatSpec { fill : ' ' , align : :: core :: fmt :: rt :: v1 :: Alignment :: Unknown , flags : 0u32 , precision : :: core :: fmt :: rt :: v1 :: Count :: Implied , width : :: core :: fmt :: rt :: v1 :: Count :: Implied , } , } , :: core :: fmt :: rt :: v1 :: Argument { position : 1usize , format : :: core :: fmt :: rt :: v1 :: FormatSpec { fill : ' ' , align : :: core :: fmt :: rt :: v1 :: Alignment :: Unknown , flags : 0u32 , precision : :: core :: fmt :: rt :: v1 :: Count :: Implied , width : :: core :: fmt :: rt :: v1 :: Count :: Implied , } , } , :: core :: fmt :: rt :: v1 :: Argument { position : 2usize , format : :: core :: fmt :: rt :: v1 :: FormatSpec { fill : ' ' , align : :: core :: fmt :: rt :: v1 :: Alignment :: Unknown , flags : 0u32 , precision : :: core :: fmt :: rt :: v1 :: Count :: Implied , width : :: core :: fmt :: rt :: v1 :: Count :: Implied , } , } , :: core :: fmt :: rt :: v1 :: Argument { position : 3usize , format : :: core :: fmt :: rt :: v1 :: FormatSpec { fill : ' ' , align : :: core :: fmt :: rt :: v1 :: Alignment :: Unknown , flags : 4u32 , precision : :: core :: fmt :: rt :: v1 :: Count :: Implied , width : :: core :: fmt :: rt :: v1 :: Count :: Implied , } , } ] ) ) ; } ; tmp } } ; match & wrong_slashed { tmp => { { :: std :: io :: _eprint ( :: core :: fmt :: Arguments :: new_v1_formatted ( & [ "[" , ":" , "] " , " = " , "\n" ] , & match ( & "src/main.rs" , & 331u32 , & "&wrong_slashed" , & & tmp ) { ( arg0 , arg1 , arg2 , arg3 ) => [ :: core :: fmt :: ArgumentV1 :: new ( arg0 , :: core :: fmt :: Display :: fmt ) , :: core :: fmt :: ArgumentV1 :: new ( arg1 , :: core :: fmt :: Display :: fmt ) , :: core :: fmt :: ArgumentV1 :: new ( arg2 , :: core :: fmt :: Display :: fmt ) , :: core :: fmt :: ArgumentV1 :: new ( arg3 , :: core :: fmt :: Debug :: fmt ) ] , } , & [ :: core :: fmt :: rt :: v1 :: Argument { position : 0usize , format : :: core :: fmt :: rt :: v1 :: FormatSpec { fill : ' ' , align : :: core :: fmt :: rt :: v1 :: Alignment :: Unknown , flags : 0u32 , precision : :: core :: fmt :: rt :: v1 :: Count :: Implied , width : :: core :: fmt :: rt :: v1 :: Count :: Implied , } , } , :: core :: fmt :: rt :: v1 :: Argument { position : 1usize , format : :: core :: fmt :: rt :: v1 :: FormatSpec { fill : ' ' , align : :: core :: fmt :: rt :: v1 :: Alignment :: Unknown , flags : 0u32 , precision : :: core :: fmt :: rt :: v1 :: Count :: Implied , width : :: core :: fmt :: rt :: v1 :: Count :: Implied , } , } , :: core :: fmt :: rt :: v1 :: Argument { position : 2usize , format : :: core :: fmt :: rt :: v1 :: FormatSpec { fill : ' ' , align : :: core :: fmt :: rt :: v1 :: Alignment :: Unknown , flags : 0u32 , precision : :: core :: fmt :: rt :: v1 :: Count :: Implied , width : :: core :: fmt :: rt :: v1 :: Count :: Implied , } , } , :: core :: fmt :: rt :: v1 :: Argument { position : 3usize , format : :: core :: fmt :: rt :: v1 :: FormatSpec { fill : ' ' , align : :: core :: fmt :: rt :: v1 :: Alignment :: Unknown , flags : 4u32 , precision : :: core :: fmt :: rt :: v1 :: Count :: Implied , width : :: core :: fmt :: rt :: v1 :: Count :: Implied , } , } ] ) ) ; } ; tmp } } ; { :: std :: io :: _print ( :: core :: fmt :: Arguments :: new_v1 ( & [ "\u{1f4c5} Stuff going on at " , "\n" ] , & match ( & chrono :: NaiveDateTime :: from_timestamp ( < pallet_timestamp :: Module < Runtime > as frame_support :: traits :: UnixTime > :: now ( ) . as_secs ( ) as i64 , 0 ) , ) { ( arg0 , ) => [ :: core :: fmt :: ArgumentV1 :: new ( arg0 , :: core :: fmt :: Debug :: fmt ) ] , } ) ) ; } ; wrong_slashed . iter ( ) . for_each ( | w | { if correct_slashed . iter ( ) . find ( | x | x . 0 == w . 0 ) . is_some ( ) { { :: std :: io :: _print ( :: core :: fmt :: Arguments :: new_v1 ( & [ "\u{1f440} was correctly slashed\n" ] , & match ( ) { ( ) => [ ] , } ) ) ; } } else { slash_record . entry ( w . 0 . clone ( ) ) . or_default ( ) . push ( ( at , w . 1 , w . 2 ) ) ; { :: std :: io :: _print ( :: core :: fmt :: Arguments :: new_v1 ( & [ "\u{1f44e}\u{1f3fb} wrongly slashed = " , "\n" ] , & match ( & w , ) { ( arg0 , ) => [ :: core :: fmt :: ArgumentV1 :: new ( arg0 , :: core :: fmt :: Debug :: fmt ) ] , } ) ) ; } } } ) ; } ) ;
            }
            slash_record.iter().for_each(|(v, record)| {
                let sum_effective_slash = record.iter().map(|(_, x, _)| x).sum::<Balance>();
                {
                    ::std::io::_print(::core::fmt::Arguments::new_v1(
                        &["", " => Sum effective slash = ", "\n"],
                        &match (&v, &sum_effective_slash) {
                            (arg0, arg1) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt),
                                ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                            ],
                        },
                    ));
                };
            });
            remote_externalities::Builder::new()
                .module("PhragmenElection")
                .module("Balances")
                .module("System")
                .build_async()
                .await
                .execute_with(|| {
                    let mut corrupt = 0;
                    <Voting<Runtime>>::iter().for_each(|(v, _)| {
                        let reserved = Balances::reserved_balance(&v);
                        if reserved == 0 {
                            corrupt += 1;
                            {
                                ::std::io::_print(::core::fmt::Arguments::new_v1(
                                    &["\u{274c} corrupt account = ", "\n"],
                                    &match (&v,) {
                                        (arg0,) => [::core::fmt::ArgumentV1::new(
                                            arg0,
                                            ::core::fmt::Display::fmt,
                                        )],
                                    },
                                ));
                            };
                            {
                                ::std::io::_print(::core::fmt::Arguments::new_v1(
                                    &["\u{1f4d5} Slash records\n"],
                                    &match () {
                                        () => [],
                                    },
                                ));
                            };
                            slash_record.entry(v).or_default().iter().for_each(|r| {
                                ::std::io::_print(::core::fmt::Arguments::new_v1(
                                    &["\t", "\n"],
                                    &match (&r,) {
                                        (arg0,) => [::core::fmt::ArgumentV1::new(
                                            arg0,
                                            ::core::fmt::Debug::fmt,
                                        )],
                                    },
                                ));
                            });
                        }
                    });
                    match corrupt {
                        tmp => {
                            {
                                ::std::io::_eprint(::core::fmt::Arguments::new_v1_formatted(
                                    &["[", ":", "] ", " = ", "\n"],
                                    &match (&"src/main.rs", &386u32, &"corrupt", &&tmp) {
                                        (arg0, arg1, arg2, arg3) => [
                                            ::core::fmt::ArgumentV1::new(
                                                arg0,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg1,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg2,
                                                ::core::fmt::Display::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                arg3,
                                                ::core::fmt::Debug::fmt,
                                            ),
                                        ],
                                    },
                                    &[
                                        ::core::fmt::rt::v1::Argument {
                                            position: 0usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 1usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 2usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 0u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                        ::core::fmt::rt::v1::Argument {
                                            position: 3usize,
                                            format: ::core::fmt::rt::v1::FormatSpec {
                                                fill: ' ',
                                                align: ::core::fmt::rt::v1::Alignment::Unknown,
                                                flags: 4u32,
                                                precision: ::core::fmt::rt::v1::Count::Implied,
                                                width: ::core::fmt::rt::v1::Count::Implied,
                                            },
                                        },
                                    ],
                                ));
                            };
                            tmp
                        }
                    };
                });
        }
    }
    async_std::task::block_on(async { main().await })
}
