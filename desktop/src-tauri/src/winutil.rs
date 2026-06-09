//! Small Windows helpers shared across backends: running a PowerShell command
//! elevated through UAC, and the base64 needed to pass it safely.

use std::process::Command;

/// Run a PowerShell command elevated (UAC). Returns Ok only if the elevated
/// child exits 0. `Start-Process -Verb RunAs` prompts when we're not already
/// elevated and runs silently when we are. A cancelled prompt is reported clearly.
pub fn run_elevated(action: &str) -> Result<(), String> {
    // Pass the inner command as a base64 -EncodedCommand to sidestep all nested
    // quoting through Start-Process -ArgumentList.
    let encoded = encode_command(action);
    let launcher = format!(
        "try {{ $p = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden \
         -ArgumentList '-NoProfile','-NonInteractive','-EncodedCommand','{encoded}'; \
         exit $p.ExitCode }} catch {{ exit 1223 }}"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &launcher])
        .output()
        .map_err(|e| format!("failed to launch elevated PowerShell: {e}"))?;
    match out.status.code() {
        Some(0) => Ok(()),
        Some(1223) => {
            Err("Elevation cancelled — approve the Administrator (UAC) prompt to continue".into())
        }
        Some(c) => Err(format!("Action failed with administrator rights (exit code {c})")),
        None => Err("Elevated action terminated unexpectedly".into()),
    }
}

/// Encode a PowerShell command for `-EncodedCommand` (base64 of UTF-16LE).
fn encode_command(ps: &str) -> String {
    let utf16le: Vec<u8> = ps.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    base64(&utf16le)
}

/// Minimal standard-alphabet base64 (no external crate needed).
fn base64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
