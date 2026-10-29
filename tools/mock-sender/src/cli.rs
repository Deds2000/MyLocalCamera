//! CLI do `mock-sender` (clap derive).

use std::path::PathBuf;

use clap::Parser;

use crate::session::MockConfig;
use crate::state_file;
use crate::uri::parse_pairing_uri;

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
        })
    }
}
