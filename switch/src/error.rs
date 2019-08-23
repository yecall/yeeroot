//! Initialization errors.

use error_chain::*;

error_chain! {
	foreign_links {
		Io(::std::io::Error) #[doc="IO error"];
	}
	links {
	}
}
