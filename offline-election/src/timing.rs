/// Start timing with the given name.
#[macro_export]
macro_rules! t_start {
	($name:ident) => {
		let $name = std::time::Instant::now();
	};
}

/// Stop and print timing with the given name.
#[macro_export]
macro_rules! t_stop {
	($name:tt) => {
		log::debug!(
			target: crate::LOG_TARGET,
			"⏳ {} took {}ms.",
			stringify!($name),
			$name.elapsed().as_millis(),
		)
	};
}
