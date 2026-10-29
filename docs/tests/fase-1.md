# Relatório de testes — Fase 1

| Campo | Valor |
|---|---|
| Data | 2026-10-29 |
| Commit / tag | `f14d24b` (último antes do portão) + commit do portão (este relatório) |
| Versão do desktop · APK · protocolo | protocolo v1.0 (`docs/protocol.md`) · desktop: sem build de release (Fase 1, sem UI final) · APK: não existe ainda (Fase 2) |
| PC (CPU · GPU · RAM · Windows build) | Intel(R) Core(TM) i7-10700F CPU @ 2.90GHz · NVIDIA GeForce RTX 3090 · 64 GB · Windows 10.0.26300 |
| Celular (modelo · Android · SoC) | não utilizado — o `tools/mock-sender` roda no mesmo PC |
| Rede (roteador · banda · USB) | loopback 127.0.0.1 (Wi-Fi/USB fora do escopo da Fase 1) |
| Responsável | agente (preparo do portão F1-GATE) |

## Resultados

Todos os 70 IDs da Fase 1 (26 UT · 8 IT · 15 AT · 10 FT · 3 PT · 6 ST · 2 LT) estão mapeados abaixo.
**Nenhum ID ficou sem teste ou medição.** Onde o ID não tem teste com o próprio nome, o cenário
está coberto por outro teste (registrado em `PROGRESS.md`, observação F1-T13): IT-1.02/AT-1.04,
IT-1.06/AT-1.08 e AT-1.01 (vetores).

| ID | Tipo | Resultado (OK / FALHA / N/A) | Medição | Evidência | Observações |
|---|---|---|---|---|---|
| UT-1.01 | UT | OK | vetores `frames` 100% (3 modos de entrega) | `crates/lc-protocol/tests/vectors.rs` → `vectors_frames_all_delivery_modes` | |
| UT-1.02 | UT | OK | vetores `json` 100% em TS | `packages/shared/src/protocol/schemas.test.ts` → `json_vectors_pass` | |
| UT-1.03 | UT | OK | round-trip `pts` 0 e `u64::MAX`, `length` no limite | `crates/lc-protocol/src/frame.rs` → `roundtrip_pts_extremes` | |
| UT-1.04 | UT | OK | `BadMagic` | `crates/lc-protocol/src/frame.rs` → `bad_magic_errors` | |
| UT-1.05 | UT | OK | 8 MiB aceito; 8 MiB+1 → `TooLarge` antes de alocar | `crates/lc-protocol/src/frame.rs` → `too_large_before_alloc` | |
| UT-1.06 | UT | OK | tipo `0x99` pulado; válidos entregues | `crates/lc-protocol/src/message.rs` → `unknown_type_becomes_unknown` + vetor `unknown_type_then_ping` em `tests/vectors.rs` | |
| UT-1.07 | UT | OK | campos extras ignorados | `crates/lc-protocol/src/message.rs` → `extra_fields_ignored` | |
| UT-1.08 | UT | OK | obrigatório ausente/tipo errado → `BadJson` | `crates/lc-protocol/src/message.rs` → `missing_required_is_badjson` | |
| UT-1.09 | UT | OK | `reason` desconhecido → `Other`, sem erro | `crates/lc-protocol/src/message.rs` → `unknown_reason_is_other` (+ `tests/vectors_json.rs`) | |
| UT-1.10 | UT | OK | fragmentação em todas as divisões (2 partes) do vetor; posições 1..N em geral cobertas pelo proptest UT-1.20 | `crates/lc-protocol/tests/vectors.rs` → `fragment_every_position` | |
| UT-1.11 | UT | OK | quadros colados no mesmo `push` | `crates/lc-protocol/src/frame.rs` → `glued_frames` | |
| UT-1.12 | UT | OK | 10 000 tokens (32 hex) e chaves (64 hex): só hex, sem colisão | `crates/lc-server/src/trust/secrets.rs` → `generated_secrets_shape_and_uniqueness` | |
| UT-1.13 | UT | OK | arquivo guarda só o hash da chave | `crates/lc-server/tests/trust.rs` → `only_hash_is_persisted` | |
| UT-1.14 | UT | OK | comparação em tempo constante sem panic (tamanhos 0–1000) | `crates/lc-server/src/trust/secrets.rs` → `ct_eq_different_lengths_no_panic` | |
| UT-1.15 | UT | OK | tabela de transições da conexão | `crates/lc-server/src/state.rs` → `state_transitions_table` | |
| UT-1.16 | UT | OK | `approve_always`/`approve_once`/`reject`/timeout (relógio virtual) | `crates/lc-server/tests/approval.rs` → `approve_always_persists`, `approve_once_does_not_persist`, `reject_denies`, `approval_timeout_denies` | |
| UT-1.17 | UT | OK | rotação de token; token antigo → `bad_token` | `crates/lc-server/tests/trust.rs` → `token_rotates_and_old_is_rejected` | |
| UT-1.18 | UT | OK | janela 15 s (inclusivo), uso único, só `127.0.0.1` | `crates/lc-server/tests/usb_window.rs` → `window_expires_after_15s`, `window_is_single_use`, `only_loopback_consumes` | |
| UT-1.19 | UT | OK | ticket uso único, TTL 10 s, preso à sessão | `crates/lc-server/src/audio/tickets.rs` → `ticket_single_use`, `ticket_expires_after_ttl`, `ticket_bound_to_session`, `new_issue_invalidates_previous` | |
| UT-1.20 | UT | OK | proptest: qualquer particionamento preserva os quadros | `crates/lc-protocol/src/frame.rs` → `prop_any_split_same_frames` | |
| UT-1.21 | UT | OK | proptest: bytes aleatórios sem panic, buffer limitado | `crates/lc-protocol/src/frame.rs` → `prop_random_bytes_never_panic` | |
| UT-1.22 | UT | OK | proptest: round-trip de `Message` válida | `crates/lc-protocol/src/message.rs` → `prop_message_roundtrip` | |
| UT-1.23 | UT | OK | matriz major/minor (1.0×1.0, 1.0×1.3, 1.3×1.0, 1.0×2.0, 0.9×1.0) | `crates/lc-server/tests/handshake.rs` → `negotiate_matrix` | |
| UT-1.24 | UT | OK | 5 falhas/60 s → bloqueio 5 min; sucesso zera; outro IP intacto; janela desliza | `crates/lc-server/tests/ratelimit.rs` → `blocks_after_max_failures`, `success_resets_counter`, `other_ip_not_affected`, `window_slides` | |
| UT-1.25 | UT | OK | JSON truncado/vazio/`schemaVersion` futuro → backup + recriação | `crates/lc-server/tests/trust.rs` → `corrupt_file_is_backed_up_and_recreated` | |
| UT-1.26 | UT | OK | falha entre escrita e rename: arquivo anterior íntegro | `crates/lc-server/src/trust/atomic.rs` → `atomic_write_keeps_old_on_failure` | |
| IT-1.01 | IT | OK | 300 quadros (faixa 295–305) e 999 blocos de áudio (faixa 980–1020) em 10 s | `crates/lc-server/tests/integration.rs` → `full_flow_10s`; detalhes em `docs/perf/fase-1.md` | |
| IT-1.02 | IT | OK | reconexão com `--device-key` aceita sem aprovação | `crates/lc-server/tests/handshake.rs` → `device_reconnect_without_key_in_welcome` (mesmo cenário de AT-1.04) | coberto conforme registro F1-T13 |
| IT-1.03 | IT | OK | 20 reconexões sem vazamento de conexões/sessões | `tools/mock-sender/tests/faults.rs` → `reconnect_20_no_leak` | |
| IT-1.04 | IT | OK | `--fragment 1`: mesmos totais (±2%) que sem fragmentar | `tools/mock-sender/tests/faults.rs` → `fragment_1_byte_same_totals` | |
| IT-1.05 | IT | OK | mesmo `deviceId`: antiga recebe `BYE{"replaced"}`, nova ativa | `crates/lc-server/tests/handshake.rs` → `same_device_replaces_session` | |
| IT-1.06 | IT | OK | 2º mock com outro `deviceId` → `busy` | `crates/lc-server/tests/handshake.rs` → `busy_for_other_device` (mesmo cenário de AT-1.08) | coberto conforme registro F1-T13 |
| IT-1.07 | IT | OK | áudio cai/reabre com novo ticket; principal intacto | `crates/lc-server/tests/audio_socket.rs` → `audio_drop_keeps_primary_and_reopens` | |
| IT-1.08 | IT | OK | eventos de stats a cada 500 ms ± 50 ms | `crates/lc-server/tests/keepalive.rs` → `stats_cadence_500ms` | |
| AT-1.01 | AT | OK | vetores em Rust e TS 100% | `crates/lc-protocol/tests/vectors.rs` + `tests/vectors_json.rs` (Rust); `packages/shared/src/protocol/schemas.test.ts` (TS) | |
| AT-1.02 | AT | OK | token correto + aprovação: aceito, `deviceKey` entregue, token rotacionado | `crates/lc-server/tests/handshake.rs` → `pair_ok_returns_device_key_and_rotates_token` | |
| AT-1.03 | AT | OK | token errado → `bad_token` e socket fechado | `crates/lc-server/tests/handshake.rs` → `bad_token_denied` | |
| AT-1.04 | AT | OK | reconexão com chave: aceita sem aprovação | `crates/lc-server/tests/handshake.rs` → `device_reconnect_without_key_in_welcome` | |
| AT-1.05 | AT | OK | dispositivo revogado reconecta → `revoked` | `crates/lc-server/tests/handshake.rs` → `revoked_device_denied` | |
| AT-1.06 | AT | OK | sem decisão no timeout → `not_approved` | `crates/lc-server/tests/approval.rs` → `approval_timeout_denies` | |
| AT-1.07 | AT | OK | `--fake-version 2.0` → `version_mismatch` + `minSupported{1,0}` | `crates/lc-server/tests/handshake.rs` → `version_mismatch_has_min_supported`; mock: `tools/mock-sender/tests/faults.rs` → `fake_version_denied` | |
| AT-1.08 | AT | OK | 2º mock com outro `deviceId` → `busy`; 1º segue ativo | `crates/lc-server/tests/handshake.rs` → `busy_for_other_device` | |
| AT-1.09 | AT | OK | áudio autenticado; blocos contados; ticket reutilizado recusado | `crates/lc-server/tests/audio_socket.rs` → `audio_ticket_flow_and_reuse_denied` | |
| AT-1.10 | AT | OK | sem `PONG` por 6 s → sessão encerrada, `connection_lost` | `crates/lc-server/tests/keepalive.rs` → `no_pong_closes_session`; mock: `tools/mock-sender/tests/faults.rs` → `no_pong_session_ends` | |
| AT-1.11 | AT | OK | `--huge-length` → `protocol_error` | `tools/mock-sender/tests/faults.rs` → `huge_length_protocol_error` | |
| AT-1.12 | AT | OK | tipo desconhecido no meio do stream: pulado, conexão segue | `tools/mock-sender/tests/faults.rs` → `unknown_type_midstream_continues` | |
| AT-1.13 | AT | OK | janela USB: `127.0.0.1` aprovado sem diálogo; fora da janela, fluxo normal | `crates/lc-server/tests/usb_window.rs` → `usb_window_auto_approves_localhost`, `outside_window_normal_flow` | |
| AT-1.14 | AT | OK | `--bad-json` no `HELLO`: conexão fechada, servidor continua | `crates/lc-server/tests/handshake.rs` → `bad_json_hello_is_protocol_error`; mock: `tools/mock-sender/tests/faults.rs` → `bad_json_closes_but_server_lives` | |
| AT-1.15 | AT | OK | restart com mesmo `data_dir`: dispositivos reconhecidos | `crates/lc-server/tests/integration.rs` → `restart_server_keeps_trusted_devices`; detalhes em `docs/perf/fase-1.md` | |
| FT-1.01 | FT | OK | *slowloris* fechado no timeout do `HELLO` | `crates/lc-server/tests/listener.rs` → `slowloris_closes_in_hello_timeout`; mock: `tools/mock-sender/tests/faults.rs` → `delay_hello_beyond_timeout` | |
| FT-1.02 | FT | OK | 20 conexões não autenticadas: limite 8 respeitado | `crates/lc-server/tests/listener.rs` → `unauth_limit_enforced` | |
| FT-1.03 | FT | OK | mock morto no meio do quadro: estado limpo, sem panic | `tools/mock-sender/tests/faults.rs` → `kill_midstream_clean` | |
| FT-1.04 | FT | OK | `HELLO` de 65 537 B (> 64 KiB) recusado antes de alocar | `crates/lc-server/tests/handshake.rs` → `huge_hello_rejected` | |
| FT-1.05 | FT | OK | porta ocupada → fallback e `PortChanged` com a porta efetiva | `crates/lc-server/tests/listener.rs` → `port_in_use_falls_back` | |
| FT-1.06 | FT | OK | conexão half-open detectada pelo keepalive | `crates/lc-server/tests/keepalive.rs` → `half_open_detected`; mock: `tools/mock-sender/tests/faults.rs` → `stall_beyond_liveness_detected` | |
| FT-1.07 | FT | OK | tempestade de conexões limitada; 2º pareamento pendente → `busy` | `crates/lc-server/tests/ratelimit.rs` → `connection_storm_limited` (+ `tests/approval.rs` → `second_pair_while_pending_is_busy`) | |
| FT-1.08 | FT | OK | mock desconecta durante o pedido: `Cancelled`, sem pendência fantasma | `crates/lc-server/tests/approval.rs` → `phone_disconnect_cancels_pending` | |
| FT-1.09 | FT | OK | `trust.json` apagado com o servidor ligado: sem crash | `crates/lc-server/tests/integration.rs` → `trust_file_deleted_while_running`; detalhes em `docs/perf/fase-1.md` | |
| FT-1.10 | FT | OK | `AUDIO_HELLO` antes do `WELCOME` → `bad_audio_ticket` | `crates/lc-server/tests/handshake.rs` → `audio_hello_before_welcome_denied`; `tests/audio_socket.rs` → `audio_hello_without_session_denied` | |
| PT-1.01 | PT | OK | 36 201 MiB/s (re-medido no portão) / 45 939 MiB/s (F1-T13) × meta 500 MiB/s (1 thread, release) | `crates/lc-protocol/tests/perf.rs` → `frame_reader_throughput_ge_500_mib_s` (`#[ignore]`; `--release --ignored --nocapture`); `docs/perf/fase-1.md` | variação de carga entre medições; ambas ≫ meta |
| PT-1.02 | PT | OK | p95 = 5,88 ms × meta 20 ms (n = 500, 0 erros; execuções 6,78/9,78/5,88 ms) | `crates/lc-server/tests/integration.rs` → `handshake_p95_under_20ms` (`#[ignore]`; `--ignored --nocapture`); `docs/perf/fase-1.md` | |
| PT-1.03 | PT | OK | média **0,32%** de 1 núcleo (máx 1,6%) × meta ≤ 3% — 30 min com 8 Mbps + áudio | `scripts/soak.ps1` + `crates/lc-server/examples/soak_server.rs`; evidência: `soak-f1.csv` + `docs/perf/fase-1.md` | derivado do CSV limpo (16 núcleos) |
| ST-1.01 | ST | OK | nenhum segredo completo nos logs (TRACE) nem no arquivo de confiança | `crates/lc-server/tests/log_redaction.rs` → `logs_never_contain_full_secrets`; `tests/trust.rs` → `debug_does_not_leak` | |
| ST-1.02 | ST | OK | 6 tentativas/min: bloqueio do IP, resposta uniforme | `crates/lc-server/tests/ratelimit.rs` → `bruteforce_6_attempts_per_minute_blocked` | |
| ST-1.03 | ST | OK | reuso de `audioTicket` e de `deviceKey` revogada recusados | `tests/trust.rs` → `token_rotates_and_old_is_rejected`, `revoke_makes_device_unknown`, `bad_key_detected`; `tests/audio_socket.rs` → `audio_ticket_flow_and_reuse_denied`; detalhes em `docs/perf/fase-1.md` | |
| ST-1.04 | ST | OK | `PING`/`SET`/`VIDEO` antes do `WELCOME` → encerrada sem resposta | `crates/lc-server/tests/handshake.rs` → `message_before_welcome_closes` | |
| ST-1.05 | ST | OK | segredos reutilizados recusados (cobertura de teste) | `tests/trust.rs` + `tests/audio_socket.rs` (conforme registro F1-T13; detalhes em `docs/perf/fase-1.md`) | parte de ferramentas (`cargo audit` + `cargo deny check licenses`): **não executado** — não aprovado pelo usuário (cartão F1-T13, passo 5: "se não aprovado, registre 'não executado'") |
| ST-1.06 | ST | OK | `--auto-approve` ignorado em release (verde em `--release`) | `tests/approval.rs` → `auto_approve_ignored_in_release`; `src/trust/approval.rs` → `auto_approve_only_in_debug`; `docs/perf/fase-1.md` | |
| LT-1.01 | LT | OK | ws 7,5 → 8,6 MB (**+1,1** × meta +5), handles 110 → 112 (**+1,8%** × meta ±5%), 0 erros nos logs | `scripts/soak.ps1 -Minutes 30` + `crates/lc-server/examples/soak_server.rs`; evidência: `soak-f1.csv` + `docs/perf/fase-1.md` | 1ª execução teve CSV corrompido (cultura pt-BR no `soak.ps1`); script corrigido e soak reexecutado limpo (180 amostras) |
| LT-1.02 | LT | N/A | item opcional do cartão F1-T13 — não executado | — | cenário de reconexão sem vazamento coberto por IT-1.03 (20 reconexões, `reconnect_20_no_leak`) |

## Defeitos encontrados
| ID | Descrição | Severidade (bloqueante/alta/média/baixa) | Situação | Teste de regressão criado |
|---|---|---|---|---|
| — | nenhum registrado na Fase 1 (ver `PROGRESS.md`, seção *Observações*, para bugs corrigidos durante o desenvolvimento, ex.: F1-T04b `claim()` no ramo `Busy`) | — | — | testes dos cartões de origem |

## Desvios aceitos
| Item | Motivo | ADR |
|---|---|---|
| Pacing do vídeo/áudio no `mock-sender`: `interval` + `MissedTickBehavior::Burst` em vez de `Delay` | granularidade do timer do Windows (~15,6 ms) com `Delay` dava ~21 fps em vez de 30; com `Burst` a média volta a ~30 fps / 200 blocos por 2 s | — (registrado em `PROGRESS.md`, observação F1-T11a) |

## Divergências protocol.md × vetores/CHEATSHEET (não corrigidas — aguardando decisão)
| Campo | Em `docs/protocol.md` | Em vetores/CHEATSHEET | Situação |
|---|---|---|---|
| Limite de tamanho do `HELLO` | não mencionado (só "Payload máximo 8 MiB", §7) | CHEATSHEET "Limites e tempos": `HELLO` ≤ 64 KiB; limite aplicado no servidor (`tests/handshake.rs::huge_hello_rejected`, 65 537 B) | ausente no protocol.md; sem contradição com vetores |
| Conexões não autenticadas simultâneas | não mencionado | CHEATSHEET "Limites e tempos": 8 | ausente no protocol.md; sem contradição |
| Bloqueio por falhas de autenticação | não mencionado | CHEATSHEET "Limites e tempos": 5 em 60 s por IP → bloqueio 5 min | ausente no protocol.md; sem contradição |
| Frequência de estatísticas | §11: "O PC envia `STATS` a cada 1 s" (mensagem PC→celular) | CHEATSHEET "Limites e tempos": "Estatísticas para a UI: no máximo a cada 500 ms (2 Hz)" (agregador local do PC) | canais distintos (wire vs UI); **não** é contradição — anotado para clareza |

Os vetores (`docs/protocol-vectors.json`) **concordam** com o protocol.md em todos os pontos
verificados: magic `43 4C`, cabeçalho 18 bytes, códigos de tipo (0x01–0x7F), payload 8 MiB /
8 MiB+1, `reason` (os 8 códigos + desconhecido), `auth.kind` (pair/device), `STATE` com `rev`,
`SET` simples e lista, `STREAM_CONFIG`, `AUDIO_HELLO`, `STATS`, `BYE`.

## Observações
- **Soak LT-1.01 concluído** (17:04–17:34, 180 amostras, máquina livre): números de PT-1.03,
  LT-1.01 e evidência em `soak-f1.csv` + `docs/perf/fase-1.md`. A 1ª execução teve o CSV
  corrompido pela cultura pt-BR no `soak.ps1` (bug do próprio script); corrigido
  (cultura invariável) e reexecutado limpo.
- **ST-1.05 (parte de ferramentas):** `cargo audit` + `cargo deny check licenses` (cartão
  F1-T13, passo 5) **não executados** — o cartão manda registrar "não executado" quando a
  instalação das ferramentas não é aprovada pelo usuário; a parte de segredos do ST-1.05
  está coberta por `tests/trust.rs` + `tests/audio_socket.rs`.
- Evidências: `soak-f1.csv` (raiz; execução limpa de 30 min) e `docs/perf/fase-1.md`
  (PT-1.01/PT-1.02/PT-1.03/ST-1.06 e detalhes de IT-1.01, AT-1.15, FT-1.09, ST-1.03).
- **Portão verde (2026-10-29):** `scripts/ci.ps1` completo OK (lint, typecheck, testes
  JS/Rust/Android, fmt, clippy — Android unit+lint 33,7 s; a 1ª execução travou num daemon
  Gradle órfão, resolvido com `gradlew --stop` e reexecução limpa, sem mudança de código);
  `pnpm run vectors:check` OK (8 vetores de frames, 24 de json); perf PT-1.01 re-medido no
  portão: **36 201 MiB/s** (anterior 45 939 — variação de carga, ambas ≫ meta 500).

## Veredito
- [x] Todos os testes obrigatórios aprovados (ou desvio aceito acima)
- [x] RS-S executada
- [x] Documentação e ADRs atualizados

**Resultado:** APROVADA (2026-10-29)
