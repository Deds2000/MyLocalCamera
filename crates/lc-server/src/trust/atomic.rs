//! Escrita atômica do `trust.json` (temporário na mesma pasta + `rename`).

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Grava `bytes` em `path` **atomicamente**: escreve em `<nome>.tmp` na
/// mesma pasta, faz `sync_all` e só então faz `rename` sobre o destino.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with(path, bytes, || Ok(()))
}

/// Mesma coisa, mas executa `hook` **entre** a escrita do temporário e o
/// `rename` (usado pelos testes para simular falha nesse intervalo).
pub(crate) fn write_atomic_with(
    path: &Path,
    bytes: &[u8],
    hook: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "caminho sem diretório"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "caminho sem nome"))?;
    let tmp = dir.join(format!("{}.tmp", name.to_string_lossy()));
    let result = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        hook()
    })();
    match result {
        Ok(()) => fs::rename(&tmp, path),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UT-1.26 — falha entre a escrita do temporário e o `rename`:
    /// o arquivo anterior continua intacto (e o temporário some).
    #[test]
    fn atomic_write_keeps_old_on_failure() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("trust.json");
        fs::write(&path, b"conteudo-anterior").unwrap();

        let err = io::Error::other("falha simulada no hook");
        let res = write_atomic_with(&path, b"conteudo-novo", || Err(err));
        assert!(res.is_err());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "conteudo-anterior",
            "arquivo anterior deve permanecer intacto"
        );
        assert!(
            !path.with_file_name("trust.json.tmp").exists(),
            "temporário deve ser removido na falha"
        );
    }
}
