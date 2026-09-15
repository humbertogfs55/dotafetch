//! Renders `art/Dota2_Official.png` natively via the Kitty terminal
//! graphics protocol (<https://sw.kovidgoyal.net/kitty/graphics-protocol/>),
//! used when the terminal supports it. `render.rs` falls back to the ANSI
//! block-art logo (`art.rs`) everywhere else.

use std::io::Write;

const PNG: &[u8] = include_bytes!("../art/Dota2_Official.png");
const PNG_WIDTH: u32 = 2000;
const PNG_HEIGHT: u32 = 2000;
/// Max base64 payload bytes per escape code chunk, per the protocol spec.
const CHUNK_LEN: usize = 4096;

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64_ALPHABET[(n >> 18 & 0x3f) as usize] as char);
        out.push(BASE64_ALPHABET[(n >> 12 & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            BASE64_ALPHABET[(n >> 6 & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64_ALPHABET[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// True when the terminal advertises Kitty graphics protocol support -
/// covers real kitty and terminals that emulate its env vars/protocol.
pub fn supported() -> bool {
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM").is_ok_and(|t| t.contains("kitty"))
}

/// Terminal cell size in pixels (width, height), queried from the kernel
/// via `TIOCGWINSZ` on stdout. Falls back to a common monospace cell
/// aspect ratio if the ioctl can't tell us (e.g. stdout isn't the tty).
fn cell_pixels() -> (f64, f64) {
    #[repr(C)]
    #[derive(Default)]
    struct Winsize {
        ws_row: libc::c_ushort,
        ws_col: libc::c_ushort,
        ws_xpixel: libc::c_ushort,
        ws_ypixel: libc::c_ushort,
    }
    let mut ws = Winsize::default();
    let ok = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) } == 0;
    if ok && ws.ws_col > 0 && ws.ws_row > 0 && ws.ws_xpixel > 0 && ws.ws_ypixel > 0 {
        (
            f64::from(ws.ws_xpixel) / f64::from(ws.ws_col),
            f64::from(ws.ws_ypixel) / f64::from(ws.ws_row),
        )
    } else {
        (10.0, 20.0)
    }
}

/// Transmits and displays the logo scaled to `target_rows` terminal rows
/// (aspect ratio preserved using the terminal's real cell size), without
/// moving the cursor (`C=1`) so the caller can lay out text over the same
/// rows. Returns the number of terminal columns the image occupies.
pub fn print(target_rows: u32) -> u32 {
    let (cell_w, cell_h) = cell_pixels();
    let px_height = f64::from(target_rows) * cell_h;
    let px_width = px_height * (f64::from(PNG_WIDTH) / f64::from(PNG_HEIGHT));
    let cols = ((px_width / cell_w).round() as u32).max(1);

    let payload = base64_encode(PNG);
    let chunks: Vec<&str> = {
        let mut v = Vec::new();
        let mut rest = payload.as_str();
        while !rest.is_empty() {
            let at = rest.len().min(CHUNK_LEN);
            let (head, tail) = rest.split_at(at);
            v.push(head);
            rest = tail;
        }
        v
    };

    let mut stdout = std::io::stdout();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        if i == 0 {
            let _ = write!(
                stdout,
                "\x1b_Ga=T,f=100,t=d,C=1,c={cols},r={target_rows},m={more};{chunk}\x1b\\"
            );
        } else {
            let _ = write!(stdout, "\x1b_Gm={more};{chunk}\x1b\\");
        }
    }
    let _ = stdout.flush();
    cols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
