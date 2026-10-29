//! CLI do `mock-sender` (clap derive).

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

use crate::faults::Faults;
use crate::session::MockConfig;
use crate::state_file;
use crate::uri::parse_pairing_uri;

/// `MS@SEG` → `(pausa, após)` do `--stall`.
fn parse_stall(raw: &str) -> Result<(Duration, Duration), String> {
    let (ms, seg) = raw
        .split_once('@')
        .ok_or_else(|| "use o formato MS@SEG (ex.: 2000@1)".to_owned())?;
    let ms: u64 = ms
        .parse()
        .map_err(|_| format!("milissegundos inválidos: {ms}"))?;
    let seg: f64 = seg
        .parse()
        .map_err(|_| format!("segundos inválidos: {seg}"))?;
    if !seg.is_finite() || seg < 0.0 {
        return Err(format!("segundos inválidos: {seg}"));
    }
    Ok((Duration::from_millis(ms), Duration::from_secs_f64(seg)))
}

/// `M.m` → `(major, minor)` do `--fake-version`.
fn parse_version(raw: &str) -> Result<(u16, u16), String> {
    let (major, minor) = raw
        .split_once('.')
        .ok_or_else(|| "use o formato M.m (ex.: 2.0)".to_owned())?;
    Ok((
        major
            .parse()
            .map_err(|_| format!("major inválido: {major}"))?,
        minor
            .parse()
            .map_err(|_| format!("minor inválido: {minor}"))?,
    ))
}

/// Finge ser o celular LocalCamera: pareia, reconecta e envia vídeo/áudio.
#[derive(Debug, Parser)]
#[command(
    name = "mock-sender",
    about = "Mock do celular LocalCamera (pareamento, vídeo H.264 e áudio)"
)]
pub struct Cli {
    /// URI de pareamento (`localcamera://connect?...`).
    #[arg(long, conflicts_with_all = ["host", "token"])]
    pub uri: Option<String>,
    /// Host do PC (use com `--token` ou com um estado já pareado).
    #[arg(long, conflicts_with = "uri")]
    pub host: Option<String>,
    /// Porta do servidor (ignorada quando `--uri` traz a sua).
    #[arg(long, default_value_t = 48650)]
    pub port: u16,
    /// Token de pareamento (32 hex).
    #[arg(long)]
    pub token: Option<String>,
    /// ID do dispositivo (padrão: do estado salvo ou um UUID novo).
    #[arg(long)]
    pub device_id: Option<String>,
    /// Nome exibido do celular.
    #[arg(long, default_value = "Mock Phone")]
    pub name: String,
    /// Arquivo de estado `{deviceId, deviceKey}` (salvo após parear).
    #[arg(long)]
    pub state_file: Option<PathBuf>,
    /// Chave de dispositivo já pareado (dispensa `--token`).
    #[arg(long)]
    pub device_key: Option<String>,
    /// Arquivo `.h264` (Annex-B) a enviar.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Reinicia o vídeo do começo ao acabar o arquivo.
    #[arg(long = "loop")]
    pub loop_video: bool,
    /// Quadros por segundo do envio.
    #[arg(long, default_value_t = 30)]
    pub fps: u32,
    /// Abre o socket de áudio e envia um seno.
    #[arg(long)]
    pub audio: bool,
    /// Frequência do seno, em Hz.
    #[arg(long, default_value_t = 440.0)]
    pub tone: f32,
    /// Duração em segundos (sem valor: até o arquivo acabar ou Ctrl+C).
    #[arg(long)]
    pub seconds: Option<f64>,

    // ── Falhas determinísticas (F1-T11b) ──────────────────────────────
    /// Todos os `write` saem em pedaços de N bytes (flush entre eles).
    #[arg(long, value_name = "N")]
    pub fragment: Option<usize>,
    /// Primeiro quadro com `magic` errado.
    #[arg(long)]
    pub bad_magic: bool,
    /// Primeiro quadro com `length = 8 MiB + 1` (só o cabeçalho).
    #[arg(long)]
    pub huge_length: bool,
    /// `HELLO` com JSON inválido.
    #[arg(long)]
    pub bad_json: bool,
    /// Insere um quadro de tipo `0x99` a cada 30 quadros de vídeo.
    #[arg(long)]
    pub unknown_type: bool,
    /// Espera MS ms antes do `HELLO`.
    #[arg(long, value_name = "MS")]
    pub delay_hello: Option<u64>,
    /// Não responde `PING`.
    #[arg(long)]
    pub no_pong: bool,
    /// Descarta o 1º `PING` e responde os demais.
    #[arg(long)]
    pub drop_ping: bool,
    /// Limita a vazão de escrita a KBPS kbit/s.
    #[arg(long, value_name = "KBPS")]
    pub slow: Option<u32>,
    /// Depois de SEG s, para de escrever por MS ms (`MS@SEG`).
    #[arg(long, value_name = "MS@SEG", value_parser = parse_stall)]
    pub stall: Option<(Duration, Duration)>,
    /// Fecha o socket abruptamente (sem `BYE`) depois de SEG s.
    #[arg(long, value_name = "SEG")]
    pub kill_after: Option<f64>,
    /// Conecta/envia 1 s/desconecta, N vezes (usa `--state-file`).
    #[arg(long, value_name = "N")]
    pub reconnect: Option<u32>,
    /// Anuncia `protocol {major: M, minor: m}` no `HELLO` (`M.m`).
    #[arg(long, value_name = "M.m", value_parser = parse_version)]
    pub fake_version: Option<(u16, u16)>,
}

impl Cli {
    /// Valida e converte a CLI na configuração da sessão.
    ///
    /// # Errors
    /// Mensagem legível para erro de argumentos.
    pub fn into_config(self) -> Result<MockConfig, String> {
        let state = self.state_file.as_deref().and_then(state_file::load);

        let mut hosts = Vec::new();
        let mut port = self.port;
        let mut token = self.token.clone();
        if let Some(uri) = &self.uri {
            let parsed = parse_pairing_uri(uri).map_err(|e| format!("--uri: {e}"))?;
            hosts = parsed.hosts.iter().map(ToString::to_string).collect();
            port = parsed.port;
            token = Some(parsed.token);
        } else if let Some(host) = &self.host {
            hosts.push(host.clone());
        } else {
            return Err("informe --uri ou --host".to_owned());
        }

        let device_id = self
            .device_id
            .clone()
            .or_else(|| state.as_ref().map(|s| s.device_id.clone()))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let device_key = self
            .device_key
            .clone()
            .or_else(|| state.as_ref().map(|s| s.device_key.clone()));

        if device_key.is_none() && token.is_none() {
            return Err(
                "informe --token (ou use --state-file/--device-key de um pareamento)".to_owned(),
            );
        }
        if self.fps == 0 {
            return Err("--fps deve ser >= 1".to_owned());
        }
        if let Some(secs) = self.seconds {
            if !secs.is_finite() || secs <= 0.0 {
                return Err("--seconds deve ser > 0".to_owned());
            }
        }
        if !self.tone.is_finite() || self.tone <= 0.0 {
            return Err("--tone deve ser > 0".to_owned());
        }
        if self.fragment == Some(0) {
            return Err("--fragment deve ser >= 1".to_owned());
        }
        if self.slow == Some(0) {
            return Err("--slow deve ser >= 1".to_owned());
        }
        if let Some(secs) = self.kill_after {
            if !secs.is_finite() || secs <= 0.0 {
                return Err("--kill-after deve ser > 0".to_owned());
            }
        }
        if self.reconnect == Some(0) {
            return Err("--reconnect deve ser >= 1".to_owned());
        }

        Ok(MockConfig {
            hosts,
            port,
            token,
            device_id,
            device_name: self.name,
            device_key,
            state_file: self.state_file,
            file: self.file,
            loop_video: self.loop_video,
            fps: self.fps,
            audio: self.audio,
            tone: self.tone,
            seconds: self.seconds,
            faults: Faults {
                fragment: self.fragment,
                bad_magic: self.bad_magic,
                huge_length: self.huge_length,
                bad_json: self.bad_json,
                unknown_type: self.unknown_type,
                delay_hello: self.delay_hello.map(Duration::from_millis),
                no_pong: self.no_pong,
                drop_ping: self.drop_ping,
                slow_kbps: self.slow,
                stall: self.stall,
                kill_after: self.kill_after.map(Duration::from_secs_f64),
                reconnect: self.reconnect,
                fake_version: self.fake_version,
            },
        })
    }
}
