//! Substrate Node Template CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

pub use yee_cli::{VersionInfo, IntoExit, error};

fn run() -> yee_cli::error::Result<()> {
	let version = VersionInfo {
		name: "Yee Root Node",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "YeeRoot",
		author: "YeeCo",
		description: "Yee Root Node",
		support_url: "contact@yeefoundation.com",
	};
	yee_cli::run(::std::env::args(), yee_cli::Exit, version)
}

error_chain::quick_main!(run);
