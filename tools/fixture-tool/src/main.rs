//! Inspeciona e manipula fixtures H.264 Annex-B (info, concat, corrupt, drop-first-au).
use std::env;
use std::fs;
use std::io::Write;
use std::process::ExitCode;

use lc_testkit::annexb;

const USAGE: &str = "uso:
  fixture-tool info <in>
  fixture-tool concat <out> <in1> <in2> ...
  fixture-tool corrupt <in> <out> <au_index>
  fixture-tool drop-first-au <in> <out>";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("erro: {msg}");
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let (cmd, rest) = args
        .split_first()
        .ok_or_else(|| "falta o subcomando".to_string())?;
    match cmd.as_str() {
        "info" => {
            if rest.len() != 1 {
                return Err("info: esperava exatamente 1 argumento (<in>)".into());
            }
            info(&rest[0])
        }
        "concat" => {
            if rest.len() < 2 {
                return Err("concat: esperava <out> <in1> <in2> ...".into());
            }
            concat(&rest[0], &rest[1..])
        }
        "corrupt" => {
            if rest.len() != 3 {
                return Err("corrupt: esperava <in> <out> <au_index>".into());
            }
            let idx: usize = rest[2]
                .parse()
                .map_err(|_| format!("corrupt: índice inválido: {}", rest[2]))?;
            corrupt(&rest[0], &rest[1], idx)
        }
        "drop-first-au" => {
            if rest.len() != 2 {
                return Err("drop-first-au: esperava <in> <out>".into());
            }
            drop_first_au(&rest[0], &rest[1])
        }
        other => Err(format!("subcomando desconhecido: {other}")),
    }
}

fn info(path: &str) -> Result<(), String> {
    let data = read(path)?;
    let aus = annexb::split_access_units(&data);
    let idrs: Vec<String> = aus
        .iter()
        .enumerate()
        .filter(|(_, r)| annexb::is_idr(&data[(*r).clone()]))
        .map(|(i, _)| i.to_string())
        .collect();
    println!("AUs: {}", aus.len());
    println!("IDRs: {}", idrs.join(", "));
    Ok(())
}

fn concat(out: &str, inputs: &[String]) -> Result<(), String> {
    let mut f = fs::File::create(out).map_err(|e| format!("ao criar {out}: {e}"))?;
    for in_path in inputs {
        let data = read(in_path)?;
        f.write_all(&data)
            .map_err(|e| format!("ao escrever {out} (input {in_path}): {e}"))?;
    }
    Ok(())
}

fn corrupt(in_path: &str, out_path: &str, au_index: usize) -> Result<(), String> {
    let data = read(in_path)?;
    let aus = annexb::split_access_units(&data);
    let Some(au) = aus.get(au_index) else {
        return Err(format!(
            "corrupt: AU {au_index} não existe (total: {})",
            aus.len()
        ));
    };
    if au.len() < 100 {
        return Err(format!(
            "corrupt: AU {au_index} tem {} bytes (mínimo 100 para corromper)",
            au.len()
        ));
    }
    let mut out = data.clone();
    let off = au.start + 40;
    for b in &mut out[off..off + 32] {
        *b = 0xFF;
    }
    fs::write(out_path, out).map_err(|e| format!("ao escrever {out_path}: {e}"))
}

fn drop_first_au(in_path: &str, out_path: &str) -> Result<(), String> {
    let data = read(in_path)?;
    let aus = annexb::split_access_units(&data);
    if aus.len() < 2 {
        return Err(format!(
            "drop-first-au: arquivo tem {} AUs (precisa de 2+)",
            aus.len()
        ));
    }
    let mut out = Vec::with_capacity(data.len());
    for au in aus.iter().skip(1) {
        out.extend_from_slice(&data[(*au).clone()]);
    }
    fs::write(out_path, out).map_err(|e| format!("ao escrever {out_path}: {e}"))
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("ao ler {path}: {e}"))
}
