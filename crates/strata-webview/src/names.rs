//! File names offered by a site, made safe for this PC. The rule lives in `strata_types::file_name` so that
//! the code that checks a site's file list against it (`strata_net::ngii_auto`) uses the very same function.

pub use strata_types::file_name::safe_file_name;
