const fs = require('fs')
const path = require('path')
const dirs = ["remote-externalities", "offline-election", "sub-storage", "sub-du"]

const VERSION_TYPE = {
	EXACT: "2.0.0-rc4",
	GIT: "master",
	LOCAL: "",
}

const SPECIAL_VERSIONS = {
	"frame-metadata": "11.0.0-rc4",
	"sp-externalities": "0.8.0-rc4",
};

function set_exact(package, version, with_optional) {
	if (SPECIAL_VERSIONS[package]) {
		version = SPECIAL_VERSIONS[package]
		console.log(`Overriding ${version} for ${package}`)
	}
	return `${package} = { version = "${version}"${with_optional ? ", optional = true " : " "}}\n`
}

function set_git(package, branch, with_optional) {
	return `${package} = { git = "https://github.com/paritytech/substrate", branch = "${branch}"${with_optional ? ", optional = true " : " "}}\n`
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
				case VERSION_TYPE.GIT :
					output += set_git(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.LOCAL :
					let primitive_package = package.split("-").slice(1).join("-")
					output += set_local(package, "primitives", primitive_package, line.includes("optional"))
					break
			}
		} else if (line.startsWith("frame-") || line.startsWith("pallet-")) {
			let package = line.split(" ")[0]
			switch(version) {
				case VERSION_TYPE.EXACT :
					output += set_exact(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.GIT :
					output += set_git(package, version, line.includes("optional"))
					break
				case VERSION_TYPE.LOCAL :
					let frame_package = package.split("-").slice(1).join("-")
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
		fs.writeFileSync(cargo_file, new_content)
	}
}

console.log(process.argv[2])
switch(process.argv[2]) {
	case "git":
		main(VERSION_TYPE.GIT)
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

