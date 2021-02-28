use codec::Encode;
use hex_literal::hex;
use structopt::StructOpt;
use sub_storage::helpers;

#[async_std::main]
async fn main() {
	let client = sub_storage::create_ws_client("ws://localhost:9944").await;

	let mut now = sub_storage::get_head(&client).await;
	let w = sub_storage::read::<frame_system::ConsumedWeight>(
		sub_storage::value_key(b"System", b"BlockWeight"),
		&client,
		now,
	)
	.await;
	dbg!(w);
}
