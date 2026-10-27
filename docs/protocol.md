# Protocolo LocalCamera — v1 (rascunho a ser finalizado na Fase 1)

Esta é a **fonte da verdade** do protocolo. Toda mudança aqui exige atualizar `docs/protocol-vectors.json` e os três lados (Rust, TypeScript, Kotlin) **no mesmo commit**.

## 1. Princípios

- O **PC é o servidor de rede**; o celular sempre conecta nele.
- O **celular é dono do estado** da câmera. Qualquer lado pede mudanças; o celular aplica na ordem de chegada (último comando vence) e publica o resultado.
- Latência antes de tudo: sem jitter buffer de vídeo; quadros velhos são descartados, não enfileirados.
- Sem criptografia do tráfego (decisão de uso próprio). A autenticação evita conexões acidentais ou não aprovadas, **não protege contra quem escuta a rede**.

## 2. Versionamento e compatibilidade

- `protocol = { major, minor }`. Versão atual: **1.0**.
- **Major diferente** → recusa: `WELCOME { accepted:false, reason:"version_mismatch", minSupported }`. As duas UIs mostram "Atualize o app do celular" ou "Atualize o app do PC", conforme o caso, com as versões dos dois lados.
- **Minor diferente** → funciona. Regras de compatibilidade:
  - Campos JSON desconhecidos são **ignorados** (serde `#[serde(default)]`, sem `deny_unknown_fields`).
  - Mensagens de `type` desconhecido são **puladas** usando `length` e registradas em log (nunca encerram a conexão).
  - Recursos novos só são usados se anunciados em `caps`.
- `HELLO.app.version` e `WELCOME.pc.appVersion` (semver dos aplicativos) servem só para diagnóstico e para as mensagens de "atualize".

## 3. Conexões (dois sockets TCP)

Mesma porta (padrão `48650`), `TCP_NODELAY` nos dois.

| Socket | Conteúdo | Primeira mensagem |
|---|---|---|
| **Principal** | controle + vídeo (streams 0, 1 e 2) | `HELLO` |
| **Áudio** | áudio (stream 3) | `AUDIO_HELLO` |

Motivo: dentro de um único TCP, um keyframe grande atrasa o áudio que vem depois dele (bloqueio de cabeça de fila), e falhas de áudio são muito mais perceptíveis que quedas de vídeo. Com sockets separados, o áudio tem buffers próprios.

- O socket de áudio só é aberto **depois** do `WELCOME` do principal, e só se `audio.enabled` for verdadeiro.
- Autenticação do áudio: o `WELCOME` traz `audioTicket` (aleatório, 128 bits, uso único, validade de 10 s). O celular envia `AUDIO_HELLO { sessionId, ticket }`; o PC responde `WELCOME { accepted }`.
- Se o socket de áudio cair, o principal continua; o celular reabre o de áudio com um novo ticket obtido por `AUDIO_TICKET_REQUEST` (ver §8).
- Opcional, melhor esforço: marcar o tráfego (`IP_TOS`) — áudio com prioridade maior que vídeo. Roteadores podem ignorar; não depender disso.
- Em USB, `adb reverse` encaminha as duas conexões (mesma porta).
- **Um celular por vez.** Nova conexão do mesmo `deviceId` substitui a anterior; de outro dispositivo recebe `reason:"busy"`.

## 4. URI de pareamento (QR code, deep link USB, digitação manual)

```
localcamera://connect?v=1&hosts=192.168.0.10,172.20.10.2&port=48650&token=<32 hex>&pc=<pcId>&name=<nome do PC>
```

- `hosts`: IPs do PC; o celular tenta todos em paralelo (timeout 2 s) e usa o primeiro que responder.
- `token`: **token de pareamento**. Serve só para parear um dispositivo novo. Roda a cada pareamento bem-sucedido e quando o usuário clica em "Regenerar".
- `pc`: `pcId` (UUID gerado na instalação do desktop), usado para reconhecer o PC no mDNS.
- O app Android registra intent-filter para `localcamera://`; `@capacitor/app` entrega `appUrlOpen` ao React. O QR lido pela câmera nativa do Android também abre o app.

## 5. Descoberta (mDNS)

- O desktop anuncia `_localcamera._tcp.local.` (crate sugerida: `mdns-sd`):
  - instância = nome do PC;
  - TXT: `v=1`, `id=<pcId>`, `name=<nome>`, `port=<porta>`;
  - **sem segredos** no TXT.
- O celular (Android `NsdManager`) procura pelo `pcId` salvo, resolve o IP e conecta usando a **chave do dispositivo** (§6).
- Cadeia de reconexão: **mDNS → últimos `hosts` conhecidos → pedir QR/endereço manual**.
- Nada de mDNS por USB (`127.0.0.1` direto).

## 6. Handshake, autenticação e aprovação

### 6.1 Dois tipos de credencial

| `auth.kind` | Quando | O que valida |
|---|---|---|
| `pair` | primeira conexão de um dispositivo | `token` de pareamento do QR |
| `device` | conexões seguintes | `deviceId` + `key` (chave por dispositivo, 256 bits, entregue no pareamento) |

O PC guarda só o **hash SHA-256** da chave de cada dispositivo (`trusted_devices[]`: `deviceId`, `name`, `keyHash`, `firstSeen`, `lastSeen`, `lastIp`). O celular guarda a chave em armazenamento privado do app (`PairingStore`) junto com os PCs conhecidos (`pcId`, `name`, `hosts`, `port`). Comparações de segredo em **tempo constante**.

### 6.2 Fluxo de primeiro pareamento

1. Celular → `HELLO { auth: { kind:"pair", token } }`.
2. Token inválido → `WELCOME { accepted:false, reason:"bad_token" }` e fecha.
3. Token válido, dispositivo desconhecido → PC envia `PENDING { timeoutS: 30 }` e abre um **diálogo de aprovação no desktop**: "Permitir que *Pixel 8* se conecte?" com **Permitir sempre**, **Só desta vez**, **Recusar**. O celular mostra "Aguardando aprovação no PC…".
4. Aprovado → `WELCOME { accepted:true, sessionId, audioTicket, deviceKey }` (`deviceKey` só quando "Permitir sempre"). O token de pareamento é rotacionado.
5. Recusado ou sem resposta em 30 s → `WELCOME { accepted:false, reason:"not_approved" }`.

### 6.3 Conexões seguintes

`HELLO { auth: { kind:"device", deviceId, key } }`. Chave correta e dispositivo na lista → aceita sem diálogo. Dispositivo removido pelo usuário → `reason:"revoked"`; o celular apaga a chave e pede novo pareamento por QR.

### 6.4 Exceção do USB

Quando o **desktop** inicia a conexão via `adb` para um aparelho que o usuário clicou em "Conectar", abre-se uma **janela de aprovação automática de 15 s**: o primeiro `HELLO` vindo de `127.0.0.1` nessa janela é aprovado sem diálogo (o usuário já provou presença física ao plugar o cabo e clicar). Fora da janela, conexões em `127.0.0.1` seguem o fluxo normal.

### 6.5 Conteúdo do `HELLO`

```json
{
  "protocol": { "major": 1, "minor": 0 },
  "app": { "version": "0.1.0", "platform": "android" },
  "device": { "id": "uuid", "name": "Pixel 8", "model": "...", "androidApi": 35 },
  "auth": { "kind": "pair", "token": "..." },
  "caps": {
    "cameras": [
      { "id": "0", "facing": "back",  "modes": [{ "w": 1920, "h": 1080, "fps": [24, 30, 60] }] },
      { "id": "1", "facing": "front", "modes": [{ "w": 1920, "h": 1080, "fps": [30] }] }
    ],
    "concurrent": true,
    "concurrentModes": [{ "w": 1280, "h": 720, "fps": [30] }],
    "audio": true,
    "codecs": ["h264"]
  }
}
```

### 6.6 Conteúdo do `WELCOME`

```json
{ "accepted": true, "protocol": { "major": 1, "minor": 0 }, "sessionId": "...", "audioTicket": "...",
  "deviceKey": "(só no pareamento)", "pc": { "id": "pcId", "name": "DESKTOP-X", "appVersion": "0.1.0" } }
```
```json
{ "accepted": false, "reason": "version_mismatch", "minSupported": { "major": 1, "minor": 0 } }
```

**Códigos de `reason`:** `bad_token`, `not_approved`, `revoked`, `busy`, `version_mismatch`, `bad_audio_ticket`, `timeout`, `protocol_error`. As UIs traduzem por código (nunca exibem a frase vinda da rede).

## 7. Cabeçalho de quadro (18 bytes, little-endian)

| Offset | Tipo | Campo | Descrição |
|---|---|---|---|
| 0 | u16 | `magic` | `0x4C43` (bytes `43 4C`) |
| 2 | u8 | `type` | tipo (§8) |
| 3 | u8 | `flags` | bit0 = keyframe, bit1 = contém SPS/PPS |
| 4 | u8 | `stream` | 0 controle · 1 vídeo A · 2 vídeo B · 3 áudio |
| 5 | u8 | `reserved` | 0 |
| 6 | u64 | `ptsUs` | microssegundos, relógio monotônico do celular |
| 14 | u32 | `length` | tamanho do payload |

Payload máximo 8 MiB. `magic` inválido ou payload acima do limite → encerrar com `protocol_error`.

## 8. Tipos de mensagem

| Hex | Nome | Direção | Socket | Payload |
|---|---|---|---|---|
| 0x01 | `HELLO` | cel → PC | principal | JSON (§6.5) |
| 0x02 | `WELCOME` | PC → cel | ambos | JSON (§6.6) |
| 0x03 | `AUDIO_HELLO` | cel → PC | áudio | JSON `{ sessionId, ticket }` |
| 0x04 | `PENDING` | PC → cel | principal | JSON `{ timeoutS }` |
| 0x05 | `AUDIO_TICKET_REQUEST` | cel → PC | principal | vazio; PC responde `AUDIO_TICKET` |
| 0x06 | `AUDIO_TICKET` | PC → cel | principal | JSON `{ ticket }` |
| 0x10 | `VIDEO` | cel → PC | principal | access unit H.264 Annex-B |
| 0x11 | `AUDIO` | cel → PC | áudio | PCM s16le 48 kHz mono, blocos de 10–20 ms |
| 0x12 | `STREAM_CONFIG` | cel → PC | principal | JSON por stream (§10) |
| 0x20 | `SET` | PC → cel | principal | JSON `{ key, value }` (ou lista) |
| 0x21 | `STATE` | cel → PC | principal | JSON, snapshot com `rev` |
| 0x30 / 0x31 | `PING` / `PONG` | ambos | ambos | u64 timestamp local |
| 0x32 | `STATS` | PC → cel | principal | JSON: fps recebido, descartes, fila (ms) |
| 0x40 | `REQUEST_KEYFRAME` | PC → cel | principal | vazio (stream no cabeçalho) |
| 0x7F | `BYE` | ambos | ambos | JSON `{ reason }` |

## 9. Estado compartilhado

O celular guarda o estado e o altera por um único **reducer**, sempre na ordem de chegada. Mudanças feitas na UI do celular e `SET`s vindos do PC passam pelo mesmo caminho. Depois de cada mudança, o celular envia `STATE` (com `rev` incrementado). A UI do PC pode atualizar de forma otimista, mas **reconcilia com o `STATE` recebido**.

| Chave | Valores | Aplicado em |
|---|---|---|
| `camera.mode` | `single` \| `dual` | celular |
| `camera.active` | `back` \| `front` (modo `single`) | celular |
| `video.width/height/fps` | dentro das `modes` anunciadas | celular |
| `video.bitrateKbps` | número (padrão 8000 em 1080p30) | celular |
| `presentation.mirror.back/front` | boolean | **PC** |
| `presentation.rotation.back/front` | 0, 90, 180, 270 | **PC** (somada ao `rotationHint`) |
| `audio.enabled` | boolean | celular |
| `audio.gain` | 0.0–2.0 | PC |

Mapa de streams: `single` → câmera ativa no stream 1. `dual` → traseira = stream 1, frontal = stream 2.
Mudanças que reiniciam a captura geram novo `STREAM_CONFIG` + keyframe. Espelho e rotação **não** reiniciam nada.
Só locais do celular (não trafegam): idioma e comportamento em segundo plano.

## 10. Vídeo, áudio e sincronia

- **Vídeo:** H.264, perfil High (fallback Main/Baseline conforme o encoder), sem B-frames, keyframe a cada 1 s, SPS/PPS repetidos em cada keyframe, baixa latência. Padrão 8 Mbps em 1080p30; modo dual: 720p30 por stream (~4 Mbps cada), limitado ao que `caps.concurrentModes` permitir.
- **Keyframe em Annex-B** deve conter o IDR **e** os parameter sets (requisito do formato). Isso também é o que o WebCodecs espera quando não há `description`.
- **`STREAM_CONFIG`:** `{ stream, width, height, fps, rotationHint, facing, profile }`. `rotationHint` vem do `TransformationInfo` (CameraX) ou da orientação do sensor (Camera2). O PC aplica `rotationHint + rotação do usuário`.
- **Áudio:** PCM s16le 48 kHz mono sem compressão (~768 kbps). Opus no backlog.
- **Sincronia A/V:** os dois sockets usam `ptsUs` do **mesmo relógio**. O PC estima o offset na chegada e aplica um atraso manual ajustável (ms).

## 11. Fila de envio, prioridade e latência (lado do celular)

- `SendQueue` com **classes de prioridade** no socket principal: **controle** (`PING/PONG`, `SET`, `STATE`, `REQUEST_KEYFRAME`, `BYE`) sempre passa na frente de **vídeo**. O áudio tem fila e socket próprios.
- `SO_SNDBUF` pequeno (ordem de 128 KiB; confirmar empiricamente) para o atraso ficar visível na fila do app, não escondido no kernel.
- Se o atraso de vídeo na fila passar de ~150 ms: descartar quadros não-chave antigos, forçar keyframe no próximo e reduzir bitrate.
- O áudio **nunca** é descartado por causa do vídeo; se a fila de áudio passar de ~200 ms, descartar os blocos mais antigos (preferir um estalo curto a latência crescente).
- O PC envia `STATS` a cada 1 s. O celular aplica ajuste simples de bitrate (reduz ~20% com fila alta, sobe devagar quando estável) via `MediaCodec.setParameters`.
- `PING/PONG` a cada 2 s em cada socket. Sem `PONG` por 6 s → conexão perdida → reconectar (cadeia do §5).

## 12. Máquina de estados do celular

`Idle → Connecting → (Pending) → Streaming → Reconnecting → Streaming | Idle`
Erros definitivos (`revoked`, `version_mismatch`, `not_approved`) levam a `Idle` com mensagem; erros de rede levam a `Reconnecting` com backoff (1, 2, 4, 8 s) até um limite configurável.

## 13. Vetores de teste

`docs/protocol-vectors.json` contém: cabeçalhos válidos e inválidos, quadros fragmentados em vários `read`, tipos desconhecidos (devem ser pulados), JSON com campos extras (devem ser aceitos), `HELLO`/`WELCOME` de cada `reason`, e `STATE` com `rev` fora de ordem. Os três lados rodam os **mesmos** vetores.
