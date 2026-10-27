# CHEATSHEET — fatos que todo cartão assume (≈ 2k tokens)

Se este arquivo e `docs/protocol.md` divergirem, **vale `docs/protocol.md`** e você deve avisar o usuário.

## Nomes e identificadores
| Item | Valor |
|---|---|
| Produto | `LocalCamera` |
| Pacote Android / `appId` Capacitor | `com.localcamera.app` |
| Identificador Tauri | `com.localcamera.desktop` |
| Pacote compartilhado (pnpm) | `@localcamera/shared` |
| Esquema de URI | `localcamera://connect?v=1&hosts=IP1,IP2&port=48650&token=<32 hex>&pc=<pcId>&name=<nome>` |
| Porta padrão | `48650` (TCP) |
| mDNS | `_localcamera._tcp.local.` · TXT: `v`, `id`(pcId), `name`, `port` (sem segredos) |
| Câmeras virtuais | `LocalCamera` · `LocalCamera Back` · `LocalCamera Front` · microfone: `LocalCamera Microphone` |
| Protocolo | `{major:1, minor:0}` (passa a 1.1 na Fase 6) |

## Estrutura do repositório (resumo)
```
desktop/            Tauri 2 + React 19 (src/ e src-tauri/)
mobile/             Capacitor + React 19 (src/ e android/)
packages/shared/    @localcamera/shared (cn, preset Tailwind, i18n, tipos do protocolo, ui)
crates/lc-protocol/ parser/serializador Rust do protocolo
crates/lc-testkit/  utilitários de teste (porta livre, tracing, annexb)
crates/lc-server/   servidor de rede do desktop (listener, handshake, trust, áudio) — sem Tauri
tools/              mock-sender, fixture-tool, vcam-probe, audio-probe, fake-adb
virtualcam/         DLL C++ da câmera virtual
driver/audio/       microfone virtual (decisão no spike S4)
scripts/            ci.ps1, make-fixtures.ps1, dev-setup.ps1, soak.ps1
docs/               protocol.md, ui-spec.md, ui/, phases/, tasks/, spikes/, adr/, tests/
fixtures/           (fora do Git) arquivos de teste gerados
```

## Cabeçalho de quadro — 18 bytes, little-endian
| Offset | Tipo | Campo | Notas |
|---|---|---|---|
| 0 | u16 | `magic` | `0x4C43` → bytes `43 4C` |
| 2 | u8 | `type` | ver tabela abaixo |
| 3 | u8 | `flags` | bit0 = keyframe, bit1 = tem SPS/PPS |
| 4 | u8 | `stream` | 0 controle · 1 vídeo A · 2 vídeo B · 3 áudio |
| 5 | u8 | `reserved` | 0 |
| 6 | u64 | `ptsUs` | microssegundos |
| 14 | u32 | `length` | payload ≤ 8 MiB (8 388 608) |

## Tipos de mensagem
| Hex | Nome | Dir. | Socket | Payload |
|---|---|---|---|---|
| 01 | HELLO | cel→PC | principal | JSON (≤ 64 KiB) |
| 02 | WELCOME | PC→cel | ambos | JSON |
| 03 | AUDIO_HELLO | cel→PC | áudio | JSON `{sessionId, ticket}` |
| 04 | PENDING | PC→cel | principal | JSON `{timeoutS}` |
| 05 | AUDIO_TICKET_REQUEST | cel→PC | principal | vazio |
| 06 | AUDIO_TICKET | PC→cel | principal | JSON `{ticket}` |
| 10 | VIDEO | cel→PC | principal | H.264 Annex-B (1 access unit) |
| 11 | AUDIO | cel→PC | áudio | PCM s16le 48 kHz mono, 10–20 ms |
| 12 | STREAM_CONFIG | cel→PC | principal | JSON |
| 20 | SET | PC→cel | principal | JSON `{key,value}` ou lista |
| 21 | STATE | cel→PC | principal | JSON com `rev` |
| 30 / 31 | PING / PONG | ambos | ambos | u64 timestamp |
| 32 | STATS | PC→cel | principal | JSON |
| 40 | REQUEST_KEYFRAME | PC→cel | principal | vazio |
| 7F | BYE | ambos | ambos | JSON `{reason}` |
Tipo desconhecido → **pular** (usar `length`) e logar; **nunca** derrubar a conexão por isso.

## Códigos `reason` (WELCOME recusado / BYE)
`bad_token`, `not_approved`, `revoked`, `busy`, `version_mismatch`, `bad_audio_ticket`, `timeout`, `protocol_error`

## Limites e tempos
| O quê | Valor |
|---|---|
| Payload máximo | 8 MiB · `HELLO` ≤ 64 KiB |
| Primeiro `HELLO` | até 5 s |
| Conexões não autenticadas simultâneas | 8 |
| Aprovação no PC | 30 s |
| Janela de aprovação USB | 15 s, só `127.0.0.1`, uso único |
| `audioTicket` | 128 bits, uso único, 10 s |
| PING/PONG | a cada 2 s; sem PONG por 6 s = conexão perdida |
| Falhas de autenticação | 5 em 60 s por IP → bloqueio de 5 min |
| Estatísticas para a UI | no máximo a cada 500 ms (2 Hz) |
| Fila de vídeo no celular | acima de 150 ms descarta não-chave antigos |
| Fila de áudio | acima de ~200 ms descarta os mais antigos |

## Credenciais
- **Token de pareamento:** 128 bits (32 hex), vai no QR; rotaciona após cada pareamento e em "Regenerar".
- **`deviceKey`:** 256 bits (64 hex), entregue só no pareamento aprovado com "Permitir sempre". O PC guarda **só o SHA-256**.
- **`pcId`:** UUID gerado na instalação do desktop.
- `auth` no HELLO: `{kind:"pair", token}` ou `{kind:"device", deviceId, key}`.

## Estado compartilhado (o celular é o dono; `rev` incrementa)
`camera.mode` (single|dual) · `camera.active` (back|front) · `video.{width,height,fps,bitrateKbps}` · `presentation.mirror.{back,front}` · `presentation.rotation.{back,front}` (0/90/180/270) · `audio.enabled` · `audio.gain`
Mapa de streams: single → câmera ativa = stream 1. dual → traseira = 1, frontal = 2.

## Padrões
1080p · 30 fps · 8 Mbps (dual: 720p30 ~4 Mbps cada) · keyframe a cada 1 s · H.264 sem B-frames · PCM 48 kHz mono 16 bits · canvas de saída 1920×1080 NV12.

## Comandos úteis (PowerShell, raiz do repo)
```
pnpm install
pnpm lint ; pnpm typecheck ; pnpm test
pnpm run ci         # scripts/ci.ps1 completo
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1 -Fast   # rápido
cargo test --workspace
cargo run -p mock-sender -- --help
pnpm --filter desktop tauri dev
```
