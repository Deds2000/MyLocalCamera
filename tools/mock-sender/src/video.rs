//! Vídeo do mock: access units Annex-B, flags e PTS.

use std::io;
use std::ops::Range;
use std::path::Path;

use lc_testkit::annexb::{nal_types, split_access_units};

/// `ptsUs` do quadro `index` a `fps` quadros/s (`index × 1e6 / fps`).
#[must_use]
pub fn pts_for_frame(index: u64, fps: u32) -> u64 {
    index * 1_000_000 / u64::from(fps)
}

/// Flags do quadro: `1` = IDR, `+2` = o AU contém SPS **e** PPS.
#[must_use]
pub fn flags_from_au(au: &[u8]) -> u8 {
    let nals = nal_types(au);
    let mut flags = 0u8;
    if nals.contains(&5) {
        flags |= 1;
    }
    if nals.contains(&7) && nals.contains(&8) {
        flags |= 2;
    }
    flags
}

/// Lê o arquivo e separa os access units.
///
/// # Errors
/// Erro de I/O ou arquivo sem nenhum access unit (AUD/NAL 9).
pub fn load_frames(path: &Path) -> io::Result<(Vec<u8>, Vec<Range<usize>>)> {
    let data = std::fs::read(path)?;
    let aus = split_access_units(&data);
    if aus.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} não contém access units", path.display()),
        ));
    }
    Ok((data, aus))
}

#[cfg(test)]
mod tests {
    use super::{flags_from_au, pts_for_frame};

    /// UT `pts_progression`: 90 AUs a 30 fps seguem `i * 1e6 / 30`.
    #[test]
    fn pts_progression() {
        let fps = 30;
        for i in 0..90u64 {
            let pts = pts_for_frame(i, fps);
            assert_eq!(pts, i * 1_000_000 / 30);
            // O passo fixo de 33_333 µs acumula até ~i/3 µs de diferença;
            // muito abaixo de um quadro, mas não é igual byte a byte.
            assert!(
                pts.abs_diff(i * 33_333) <= 30,
                "i={i}: {pts} difere do passo fixo"
            );
        }
    }

    /// UT `flags_from_au`: AU com NAL 7,8,5 → `3`; só slice → `0`.
    #[test]
    fn flags_from_au_examples() {
        let with_params = [
            0, 0, 0, 1, 0x09, 0x10, 0, 0, 0, 1, 0x67, 0x11, 0, 0, 0, 1, 0x68, 0x12, 0, 0, 0, 1,
            0x65, 0xAA,
        ];
        assert_eq!(flags_from_au(&with_params), 3);
        let plain = [0, 0, 0, 1, 0x09, 0x30, 0, 0, 0, 1, 0x41, 0xCC];
        assert_eq!(flags_from_au(&plain), 0);
    }
}
