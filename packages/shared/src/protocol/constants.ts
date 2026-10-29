/** Constantes do protocolo (espelho do CHEATSHEET.md e de `crates/lc-protocol`). */

/** Magic little-endian; na rede aparece como `43 4C`. */
export const MAGIC = 0x4c43;

/** Tamanho do cabeçalho de quadro, em bytes. */
export const HEADER_LEN = 18;

/** Limite padrão de payload: 8 MiB. */
export const MAX_PAYLOAD = 8 * 1024 * 1024;

/** Limite do payload de HELLO: 64 KiB. */
export const HELLO_MAX = 64 * 1024;

/** Porta TCP padrão do servidor. */
export const DEFAULT_PORT = 48650;

/** Códigos de tipo de mensagem (tabela do CHEATSHEET). */
export const MSG = {
  HELLO: 0x01,
  WELCOME: 0x02,
  AUDIO_HELLO: 0x03,
  PENDING: 0x04,
  AUDIO_TICKET_REQUEST: 0x05,
  AUDIO_TICKET: 0x06,
  VIDEO: 0x10,
  AUDIO: 0x11,
  STREAM_CONFIG: 0x12,
  SET: 0x20,
  STATE: 0x21,
  PING: 0x30,
  PONG: 0x31,
  STATS: 0x32,
  REQUEST_KEYFRAME: 0x40,
  BYE: 0x7f,
} as const;

/** Códigos `reason` válidos (WELCOME recusado / BYE). */
export const REASONS = [
  "bad_token",
  "not_approved",
  "revoked",
  "busy",
  "version_mismatch",
  "bad_audio_ticket",
  "timeout",
  "protocol_error",
] as const;

/** Índices do campo `stream` do cabeçalho. */
export const STREAM = {
  control: 0,
  videoA: 1,
  videoB: 2,
  audio: 3,
} as const;

/** Tempos e limites do CHEATSHEET, em milissegundos. */
export const LIMITS = {
  /** Primeiro `HELLO` após conectar. */
  helloFirstMs: 5_000,
  /** Aprovação do dispositivo no PC. */
  approvalTimeoutMs: 30_000,
  /** Janela de aprovação USB (só 127.0.0.1, uso único). */
  usbApprovalWindowMs: 15_000,
  /** Validade do `audioTicket` (uso único). */
  audioTicketTtlMs: 10_000,
  /** Intervalo de PING. */
  pingIntervalMs: 2_000,
  /** Sem PONG neste tempo = conexão perdida. */
  pongTimeoutMs: 6_000,
  /** Janela de contagem de falhas de autenticação. */
  authFailWindowMs: 60_000,
  /** Bloqueio de IP após 5 falhas na janela. */
  authFailBlockMs: 300_000,
  /** Máximo de intervalo entre STATS para a UI (2 Hz). */
  statsIntervalMs: 500,
  /** Fila de vídeo acima disso descarta quadros não-chave antigos. */
  videoQueueDropMs: 150,
  /** Fila de áudio acima disso descarta os mais antigos. */
  audioQueueDropMs: 200,
} as const;
