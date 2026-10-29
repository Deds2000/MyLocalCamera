//! Persistência de confiança: `pcId`, token de pareamento e dispositivos
//! confiáveis (só o **SHA-256** da `deviceKey`, nunca a chave em claro).
//!
//! O arquivo `<data_dir>/trust.json` usa `schemaVersion: 1` e é gravado
//! de forma atômica (temporário na mesma pasta + `rename`). Se o arquivo
//! estiver corrompido, é copiado para `trust.json.corrupt-<ts>` e um
//! store novo é criado — o app nunca derruba por causa disso.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::atomic::write_atomic;
use super::secrets;

/// Versão atual do esquema do `trust.json`.
const SCHEMA_VERSION: u32 = 1;
/// Nome do arquivo de persistência (e base do temporário `.tmp`).
const FILE_NAME: &str = "trust.json";

/// Erros da loja de confiança.
#[derive(Debug, Error)]
pub enum TrustError {
    /// Falha no sistema de arquivos.
    #[error("falha no sistema de arquivos: {0}")]
    Io(#[from] io::Error),
    /// `trust.json` não pôde ser lido/gravado como JSON.
    #[error("trust.json inválido: {0}")]
    Json(#[from] serde_json::Error),
    /// O `device_id` não está cadastrado (ou foi revogado).
    #[error("dispositivo não cadastrado: {0}")]
    UnknownDevice(String),
}

/// Resultado da verificação da chave de um dispositivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyDevice {
    /// Cadastrado e a chave confere.
    Ok,
    /// Não cadastrado ou revogado.
    Unknown,
    /// Cadastrado, mas a chave não confere.
    BadKey,
}

/// Dispositivo aprovado com "Permitir sempre".
///
/// `key_hash` é o SHA-256 (hex) da `deviceKey`; a chave em claro nunca
/// aparece aqui nem em qualquer log.
#[derive(Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub device_id: String,
    pub name: String,
    pub key_hash: String,
    pub first_seen: u64,
    pub last_seen: u64,
    pub last_ip: String,
}

impl fmt::Debug for TrustedDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrustedDevice")
            .field("device_id", &self.device_id)
            .field("name", &self.name)
            .field("key_hash", &"<redacted>")
            .field("first_seen", &self.first_seen)
            .field("last_seen", &self.last_seen)
            .field("last_ip", &self.last_ip)
            .finish()
    }
}

/// Dados persistidos no `trust.json` (privado; nunca imprime segredos).
#[derive(Serialize, Deserialize)]
struct Data {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "pcId")]
    pc_id: String,
    #[serde(rename = "pairToken")]
    pair_token: String,
    devices: Vec<TrustedDevice>,
}

impl Data {
    fn fresh() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            pc_id: uuid::Uuid::new_v4().to_string(),
            pair_token: secrets::random_hex(16),
            devices: Vec::new(),
        }
    }
}

/// Loja de confiança (memória + `trust.json` em disco).
///
/// Todo método que altera estado grava o arquivo antes de retornar.
pub struct TrustStore {
    path: PathBuf,
    data: Data,
}

impl TrustStore {
    /// Abre a loja em `dir` (criando o diretório se preciso).
    ///
    /// Se o arquivo existir mas for inválido (JSON inválido, vazio ou
    /// `schemaVersion` desconhecido), ele é renomeado para
    /// `trust.json.corrupt-<unix_ts>` e um store novo é criado
    /// (novo `pcId`/token; dispositivos anteriores perdidos).
    pub fn open(dir: &Path) -> Result<Self, TrustError> {
        fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let data = match fs::read_to_string(&path) {
            Ok(text) if !text.trim().is_empty() => match serde_json::from_str::<Data>(&text) {
                Ok(d) if d.schema_version == SCHEMA_VERSION => d,
                _ => Self::backup_corrupt(&path)?,
            },
            // arquivo existe, mas está vazio → corrompido
            Ok(_) => Self::backup_corrupt(&path)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Data::fresh(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self { path, data })
    }

    /// Move o arquivo corrompido para `trust.json.corrupt-<ts>` e devolve
    /// dados frescos. O backup não sobrescreve um backup já existente.
    fn backup_corrupt(path: &Path) -> Result<Data, TrustError> {
        // `0` é aceitável no pior caso (relogio antes da epoch): o nome
        // do backup só precisa ser único o bastante.
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut backup = path.with_file_name(format!("{FILE_NAME}.corrupt-{ts}"));
        if backup.exists() {
            for i in 1u32.. {
                let candidate = path.with_file_name(format!("{FILE_NAME}.corrupt-{ts}-{i}"));
                if !candidate.exists() {
                    backup = candidate;
                    break;
                }
            }
        }
        fs::rename(path, &backup)?;
        Ok(Data::fresh())
    }

    /// Serializa e grava o arquivo de forma atômica.
    fn save(&self) -> Result<(), TrustError> {
        let bytes = serde_json::to_string_pretty(&self.data)?;
        write_atomic(&self.path, bytes.as_bytes())?;
        Ok(())
    }

    /// Identificador estável do PC (UUID v4).
    #[must_use]
    pub fn pc_id(&self) -> &str {
        &self.data.pc_id
    }

    /// Token de pareamento atual (32 hex).
    #[must_use]
    pub fn pair_token(&self) -> &str {
        &self.data.pair_token
    }

    /// Regenera o token de pareamento; o token antigo passa a ser recusado.
    pub fn regenerate_token(&mut self) -> Result<String, TrustError> {
        self.data.pair_token = secrets::random_hex(16);
        self.save()?;
        Ok(self.data.pair_token.clone())
    }

    /// Valida um token de pareamento (comparação em tempo constante).
    #[must_use]
    pub fn verify_pair_token(&self, token: &str) -> bool {
        secrets::ct_eq(self.data.pair_token.as_str(), token)
    }

    /// Cadastra um dispositivo confiável e devolve a `deviceKey`
    /// (64 hex) **uma única vez**. Só o SHA-256 da chave é persistido.
    ///
    /// Se o `device_id` já estiver cadastrado (ex.: parear de novo após
    /// revogação), a entrada é substituída com a chave nova.
    pub fn add_device(
        &mut self,
        device_id: &str,
        name: &str,
        ip: &str,
        now: u64,
    ) -> Result<String, TrustError> {
        let key = secrets::random_hex(32);
        let key_hash = secrets::sha256_hex(&key);
        let entry = TrustedDevice {
            device_id: device_id.to_string(),
            name: name.to_string(),
            key_hash,
            first_seen: now,
            last_seen: now,
            last_ip: ip.to_string(),
        };
        match self
            .data
            .devices
            .iter()
            .position(|d| d.device_id == device_id)
        {
            Some(i) => self.data.devices[i] = entry,
            None => self.data.devices.push(entry),
        }
        self.save()?;
        Ok(key)
    }

    /// Verifica a chave de um dispositivo (SHA-256 de `key` vs. hash salvo).
    #[must_use]
    pub fn verify_device(&self, device_id: &str, key: &str) -> VerifyDevice {
        match self.data.devices.iter().find(|d| d.device_id == device_id) {
            Some(d) => {
                if secrets::ct_eq(&d.key_hash, &secrets::sha256_hex(key)) {
                    VerifyDevice::Ok
                } else {
                    VerifyDevice::BadKey
                }
            }
            None => VerifyDevice::Unknown,
        }
    }

    /// Atualiza `last_seen`/`last_ip` de um dispositivo cadastrado.
    pub fn touch(&mut self, device_id: &str, ip: &str, now: u64) -> Result<(), TrustError> {
        let dev = self
            .data
            .devices
            .iter_mut()
            .find(|d| d.device_id == device_id)
            .ok_or_else(|| TrustError::UnknownDevice(device_id.to_string()))?;
        dev.last_seen = now;
        dev.last_ip = ip.to_string();
        self.save()
    }

    /// Revoga um dispositivo. Devolve `true` se ele estava cadastrado.
    pub fn revoke(&mut self, device_id: &str) -> Result<bool, TrustError> {
        let before = self.data.devices.len();
        self.data.devices.retain(|d| d.device_id != device_id);
        let removed = self.data.devices.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// Cópia da lista de dispositivos confiáveis (hashes, sem chaves em claro).
    #[must_use]
    pub fn list(&self) -> Vec<TrustedDevice> {
        self.data.devices.clone()
    }
}

impl fmt::Debug for TrustStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrustStore")
            .field("path", &self.path)
            .field("pc_id", &self.data.pc_id)
            .field("pair_token", &"<redacted>")
            .field(
                "devices",
                &self
                    .data
                    .devices
                    .iter()
                    .map(|d| d.device_id.as_str())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}
