
pub fn empty_ext_with_runtime<T: frame_system::Trait>() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::default().build_storage::<T>().unwrap().into()
}
