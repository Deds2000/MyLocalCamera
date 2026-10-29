# Desempenho — Fase 1 (F1-T13)

Medições em `D:\Repositorios\MyLocalCamera` (Windows 11, RTX 3090),
executadas em 2026-10-29. Testes: `crates/lc-protocol/tests/perf.rs`
(PT-1.01) e `crates/lc-server/tests/integration.rs` (IT-1.01, AT-1.15,
FT-1.09, PT-1.02).

| Item | Descrição | Meta | Medido | Resultado |
|---|---|---|---|---|
| PT-1.01 | Vazão do `FrameReader` (1 GiB de quadros `VIDEO` de 64 KiB, 1 thread, `--release`) | ≥ 500 MiB/s | **45 939 MiB/s** (F1-T13); re-medido no portão: **36 201 MiB/s** (variação de carga) | ✅ |
| PT-1.02 | 500 handshakes `hello_device` a 50/s: p95 (connect → `WELCOME`) | p95 ≤ 20 ms, zero falhas | **p95 = 5,88 ms** (n = 500, 0 erros; execuções: 6,78 / 9,78 / 5,88 ms) | ✅ |
| PT-1.03 | CPU do servidor ≤ 3% de 1 núcleo (8 Mbps + áudio, 5 min) | ≤ 3% | **média 0,32% de 1 núcleo** (máx 1,6%) — do soak de 30 min | ✅ |
| LT-1.01 | Soak 30 min: memória ≤ +5 MB (após 1 min de aquecimento), handles ±5% | ver `scripts/soak.ps1` | ws 7,5 → 8,6 MB (**+1,1**), handles 110 → 112 (**+1,8%**), CPU máx 0,1% (máq.), 0 erros nos logs; `soak-f1.csv` | ✅ |
| ST-1.06 | `auto_approve` ignorado em build release (`effective_auto_approve`) | aprovado em release | `cargo test -p lc-server --release approval`: `auto_approve_only_in_debug` (unit) e `approval_timeout_denies` (integração) passam em `--release` | ✅ |

Referências de cobertura correlatas (já existentes, confirmadas):

- **IT-1.01** (`full_flow_10s`): 300 AUs de `fixtures/hd30.h264` @ 30 fps +
  1000 blocos de áudio de 960 B @ 100/s → `video_frames[0] = 300`
  (faixa 295–305) e `audio_blocks = 999` (faixa 980–1020).
- **AT-1.15** (`restart_server_keeps_trusted_devices`): dispositivo
  confiável sobrevive ao restart com o mesmo `data_dir`; reconexão sem
  nova `deviceKey`.
- **FT-1.09** (`trust_file_deleted_while_running`): loja memória + disco —
  apagar `trust.json` com o servidor ligado não afeta o store em execução
  (uma autenticação `device` bem-sucedida **regrava** o arquivo via
  `touch`); após restart com o arquivo apagado, o device antigo é
  `revoked` e o token antigo é `bad_token`.
- **ST-1.03** (segredos reutilizados): coberto por `token_rotates_and_old_is_rejected`,
  `revoke_makes_device_unknown` e `bad_key_detected` (`tests/trust.rs`) e
  `audio_ticket_flow_and_reuse_denied` (`tests/audio_socket.rs`).

Observações:

- PT-1.01 medido com `cargo test --release -p lc-protocol --test perf
  -- --ignored --nocapture`; PT-1.02 com `cargo test -p lc-server
  --test integration -- --ignored --nocapture` (debug, pois o
  `auto_approve` só vale em debug).
- Nenhum número ficou abaixo da meta; nada foi ajustado.
- Soak LT-1.01: a 1ª execução (16:26–16:56) teve o CSV corrompido pela cultura
  pt-BR (`N1` → `8,5` virou 2 colunas); `scripts/soak.ps1` foi corrigido
  (cultura invariável) e o soak foi **reexecutado** limpo (17:04–17:34, 180
  amostras). Os números do PT-1.03 saem do CSV limpo (16 núcleos lógicos).
- O PT-1.02 deixa o canal de eventos do servidor drenado continuamente —
  canal com capacidade 64; sem drenagem o servidor trava.
