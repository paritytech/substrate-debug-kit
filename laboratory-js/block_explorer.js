const api = require('@polkadot/api')
const cliProgress = require('cli-progress');
const blessed = require('blessed');

const BLOCK_PER_HOUR = 60 * 60 / 6
const WEIGHTS_PER_SECOND = 1_000_000_000_000

async function head() {
	let last_header = await substrate.rpc.chain.getHeader()
	return last_header.number.toNumber();
}

function lastWeek(now) {
	return lastNHours(now, 7 * 24)
}

function lastDay(now) {
	return lastNHours(now, 24)
}

function lastNHours(now, n) {
	return now - (n * BLOCK_PER_HOUR)
}

async function search(from, until, query) {
	let block_weight_limit = 2 * WEIGHTS_PER_SECOND

	let now = from
	while (true) {
		let block_hash = await substrate.rpc.chain.getBlockHash(now)
		let signed_block = await substrate.rpc.chain.getBlock(block_hash)
		let block_weight = await substrate.query.system.blockWeight.at(block_hash)
		let final_block_weight = block_weight['normal'] + block_weight['operational']

		let extrinsics = signed_block.block.extrinsics
		let ext_weight_sum = 0

		let print_buffer = ""
		let ext_names = []
		for (let ext of extrinsics) {
			let info = await substrate.rpc.payment.queryInfo(ext.toHex(), block_hash)
			let weight = info['weight'].toNumber()
			print_buffer = print_buffer.concat(`\t${ext.meta.name.toString()} ==> ${weight} [${weight / block_weight_limit}]\n`)
			ext_weight_sum += weight
			ext_names.push(ext.meta.name.toString())
		}

		if (query == undefined || ext_names.findIndex(e => e.includes(query)) > -1) {
			global._output.setContent(global._output.content.concat(`++ block ${now}[${block_hash}] total weight = ${final_block_weight} ext_sum = ${ext_weight_sum} [${final_block_weight / block_weight_limit}]`))
			console.log(print_buffer)
		}

		if (now == until) { break } else { now-- }
	}
}

async function connect() {
	// let endpoint = "wss://rpc.polkadot.io"
	let endpoint = "wss://rpc.polkadot.io"
	const provider = new api.WsProvider(endpoint);
	substrate = await api.ApiPromise.create({ provider });
}

function setupScreen() {
	// Create a screen object.
	var screen = blessed.screen({
		smartCSR: true
	});

	var form = blessed.form({
		parent: screen,
		keys: true,
		left: '10px',
		top: '10px',
		width: '100%',
		height: 8,
		border: 'line' ,
		autoNext: true,
		content: 'Insert query (optional)'
	});

	var greaterThanEdit = blessed.Textbox({
		parent: form,
		top: 2,
		height: 1,
		left: 2,
		right: 2,
		bg: 'grey',
		keys: true,
		inputOnFocus: true,
		content: 'test',
	});

	var submit = blessed.button({
		parent: form,
		mouse: true,
		keys: true,
		shrink: true,
		left: 5,
		top: 4,
		width: 20,
		name: 'submit',
		content: 'submit',
		style: {
			bg: 'blue',
			focus: {
				bg: 'red'
			},
			hover: {
				bg: 'red'
			}
		}
	});

	var output = blessed.box({
		parent: screen,
		top: 8,
		width: '100%',
		height: screen.height - 8,
		border: 'line',
	})

	submit.on('press', function() {
		form.submit();
	});

	form.on('submit', function(data) {
		screen.render();
	});

	form.on('reset', function(data) {
		form.setContent('Canceled.');
		screen.render();
	});

	screen.key(['escape', 'q', 'C-c'], function(ch, key) {
		return process.exit(0);
	});

	greaterThanEdit.focus();
	screen.render();

	screen.render();

	global._screen = screen
	global._output = output
}

async function main() {
	await connect()
	let now = await head()
	setupScreen();
	search(now, lastDay(now), "submit_election_solution")
}

// (async () => {
// 	try {
// 		await main();
// 	} catch (e) {
// 		console.error(e)
// 	}
// })();

module.exports = {
	head: head,
	connect: connect,
}
