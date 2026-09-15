mod ansi;
mod art;
mod heroes;
mod image_logo;
mod locate;
mod render;
mod stats;
mod vbkv;
mod vdf;

/// `dotafetch dump <path>` - prints any of Dota's local VBKV `.dat` files
/// (stats.dat, last_match.dat, ...) as readable JSON, for poking around or
/// debugging without needing a separate tool.
fn dump(path: &str) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("dotafetch: could not read {path}: {e}");
            std::process::exit(1);
        }
    };
    match vbkv::parse(&bytes) {
        Ok(value) => {
            let json = value.to_json();
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        Err(e) => {
            eprintln!("dotafetch: {e}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(cmd) = args.next() {
        match cmd.as_str() {
            "dump" => {
                let Some(path) = args.next() else {
                    eprintln!("usage: dotafetch dump <path-to.dat>");
                    std::process::exit(1);
                };
                dump(&path);
                return;
            }
            other => {
                eprintln!("dotafetch: unknown argument {other:?}");
                std::process::exit(1);
            }
        }
    }

    let account = match locate::find_account() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("dotafetch: {e}");
            std::process::exit(1);
        }
    };

    let stats = match stats::build(&account.cfg_dir, account.install_cfg_dir.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("dotafetch: {e}");
            std::process::exit(1);
        }
    };

    render::print(&account.persona_name, &stats);
}
