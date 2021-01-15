const fs = require('fs')
const path = require('path')
const dirs = ["remote-externalities", "offline-election", "sub-storage", "sub-du", "substrate-runtime-dry-run"]

String.prototype.replaceAt = function(index, replacement) {
    return this.substr(0, index) + replacement + this.substr(index + replacement.length);
}

const VERSION_TYPE = {
	EXACT: "2.0.1",
	BRANCH: "master",
	COMMIT: "8c3b3fb1b0c858cc603444eafab4032caf6795ce",
	LOCAL: "",
}

const SPECIAL_VERSIONS = {
	"frame-metadata": "12.0.0",
	"sp-externalities": "0.8.0",
	"sp-state-machine": "0.8.1",
	"sc-executor": "0.8.1",
};

// these are folders that are not pallets themselves, but are rather sub-folders of a pallet.
const PALLET_SUB_FOLDERS = ["reward-curve"]

function set_exact(package, version, with_optional) {
	if (SPECIAL_VERSIONS[package]) {
		version = SPECIAL_VERSIONS[package]
		console.log(`Overriding ${version} for ${package}`)
	}
	return `${package} = { version = "${version}"${with_optional ? ", optional = true " : " "}}\n`
}

function set_branch(package, branch, with_optional) {
	return `${package} = { git = "https://github.com/paritytech/substrate", branch = "${branch}"${with_optional ? ", optional = true " : " "}}\n`
}

function set_commit(package, commit, with_optional) {
	return `${package} = { git = "https://github.com/paritytech/substrate", rev = "${commit}"${with_optional ? ", optional = true " : " "}}\n`
}

function set_local(package, folder, local_package, with_optional) {
	return `${package} = { path = "../../substrate/${folder}/${local_package}"${with_optional ? ", optional = true " : " "}}\n`
}

function do_update(content, version) {
	let output = ""
	for (let line of content.split("\n")) {
		if (line.startsWith("sp-")) {
			let package = line.split(" ")[0]
			switch(version) {
				case VERSION_TYPE.EXACT :
					output += set_exact(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.BRANCH :
					output += set_branch(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.COMMIT :
					output += set_commit(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.LOCAL :
					let primitive_package = package.split("-").slice(1).join("-")
					output += set_local(package, "primitives", primitive_package, line.includes("optional"))
					break
			}
		} else if (line.startsWith("sc-")) {
			let package = line.split(" ")[0]
			switch(version) {
				case VERSION_TYPE.EXACT :
					output += set_exact(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.BRANCH :
					output += set_branch(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.COMMIT :
					output += set_commit(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.LOCAL :
					let primitive_package = package.split("-").slice(1).join("-")
					output += set_local(package, "client", primitive_package, line.includes("optional"))
					break
			}
		} else if (line.startsWith("frame-") || line.startsWith("pallet-")) {
			let package = line.split(" ")[0]
			switch(version) {
				case VERSION_TYPE.EXACT :
					output += set_exact(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.BRANCH :
					output += set_branch(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.COMMIT :
					output += set_commit(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.LOCAL :
					let frame_package = package.split("-").slice(1).join("-")
					for (let s of PALLET_SUB_FOLDERS) {
						if (frame_package.includes(s)) {
							frame_package = frame_package.replaceAt(frame_package.indexOf(s)-1, "/")
							break;
						}
					}
					output += set_local(package, "frame", frame_package, line.includes("optional"))
					break
			}
		} else {
			output += (line + "\n")
		}
	}

	return output
}

function main(version) {
	for (let d of dirs) {
		let cargo_file = path.join(d, "Cargo.toml")
		let content = String(fs.readFileSync(cargo_file))
		let new_content = do_update(content, version)
		fs.writeFileSync(cargo_file, new_content.trimRight() + "\n")
	}
}

switch(process.argv[2]) {
	case "branch":
		main(VERSION_TYPE.BRANCH)
		break
	case "commit":
		main(VERSION_TYPE.COMMIT)
		break
	case "local":
		main(VERSION_TYPE.LOCAL)
		break
	case "exact":
		main(VERSION_TYPE.EXACT)
		break
	default:
		main(VERSION_TYPE.EXACT)
}

