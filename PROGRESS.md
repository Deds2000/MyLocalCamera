# PROGRESS.md — andamento do LocalCamera

> O agente atualiza este arquivo ao fim de **cada** tarefa. Legenda: `[ ]` a fazer · `[~]` em andamento · `[x]` pronta · `[!]` bloqueada.
> Executor: **A** = agente · **H** = humano (o agente para e entrega o roteiro) · **A+H** = agente prepara, humano executa a parte manual.

**Próxima tarefa:** `F2-T01` (Capacitor e deep link)  ·  **Fase atual:** 2  ·  **Última atualização:** 2026-10-29

## Bloqueios
(nenhum ativo)
- ~~F1-GATE depende do *soak* LT-1.01 (agendado)~~ → **resolvido (2026-10-29): *soak* de 30 min executado com a máquina livre.** Resultado: ws 7,5 → 8,6 MB (+1,1 × meta +5), handles 110 → 112 (+1,8% × meta ±5%), CPU média 0,32% de 1 núcleo (máx 1,6% × meta 3%), 0 erros nos logs, `soak-f1.csv` (180 amostras) = evidência. A 1ª execução teve o CSV corrompido pela cultura pt-BR no `soak.ps1` (`N1` → `8,5`); corrigido (cultura invariável, commit `f14d24b`) e reexecutado.
- ~~F0-S5 — grupos 150%/200% exigiam mudança de escala do Windows, que neste PC só vale após sign-out~~ → **resolvido por decisão do usuário (2026-10-29): pular 150/200%; veredito do S5 com 100% apenas.** (Contexto: testado que `LogPixels` + broadcast + reinício do explorer + `SPI_SETLOGPIXELSPERINCH` não aplicam a escala sem sign-out. O usuário deixou a escala em 150% nas Configurações sem aplicar — pedir para reverter a 100% para evitar surpresa em sign-out futuro.)
- **Restrição permanente:** NÃO matar os processos ffmpeg do app de compressão de vídeo do usuário (PIDs ~30096/35100, sujeitos a mudança) — ele perde o progresso da compressão.

## Observações
(o agente anota aqui problemas fora do escopo da tarefa atual)
- Cartões da Fase 2 (gerados 2026-10-29 pelo worker, auditados): 20 cartões em `docs/tasks/fase-2/` (19 tarefas + GATE), 63/63 IDs do §7 mapeados (UT 14, IT 7, AT 15, FT 14, PT 6, ST 5, LT 2), DAG sem ciclos (máx. 3 dependências; `F2-T14` é o agrupador de testes). Decisões dos cartões: `caps.concurrent` reporta o valor real e **registra a divergência** com a implementação single (Fase 2) para o usuário validar no GATE; `SO_SNDBUF` = constante 256 KiB documentada, medição de afinamento pendente no `F2-T15`; deep link: o manifest já vinha de F0-T03 (intent-filter + `singleTask` + retrato) — o `F2-T01` confirma e adiciona o `appUrlOpen`; Kotlin em subpacotes `protocol/ uri/ store/ camera/ encoder/ net/ state/ service/ plugin/` em `com.localcamera.app`.
- F1-GATE: **Fase 1 APROVADA** (relatório `docs/tests/fase-1.md`). Portão verde: `ci.ps1` completo OK (1ª execução travou num daemon Gradle órfão no passo Android — resolvido com `gradlew --stop` + reexecução limpa, sem mudança de código), `pnpm run vectors:check` OK (8 frames, 24 json), perf PT-1.01 re-medido **36 201 MiB/s** (anterior 45 939; variação de carga). `docs/protocol.md` → **v1.0** (título); vetores × protocol.md sem divergência; 3 pontos **ausentes** em protocol.md e presentes no CHEATSHEET/servidor (limite `HELLO` ≤ 64 KiB, 8 conexões não autenticadas, bloqueio 5 falhas/60 s) — listados no relatório como divergências, **não corrigidos** (cartão manda perguntar ao usuário). Cobertura 70/70 IDs sem teste nenhum pendente. README do mock-sender completo (27 flags). ST-1.05: parte de segredos coberta por testes; parte de ferramentas (`cargo audit`/`cargo deny`) **não executada** — cartão F1-T13 manda registrar "não executado" sem aprovação do usuário. `soak-f1.csv` (execução limpa de 30 min) commitado como evidência. **Pendências p/ usuário:** (a) decisão sobre os 3 pontos ausentes em `protocol.md`; (b) aprovar instalação de `cargo audit`/`cargo deny` se quiser a parte de ferramentas do ST-1.05.
- F1-T13: trabalho do subagente local auditado linha a linha antes do commit (escopo = 3 arquivos novos; nada fora do escopo tocado). IT-1.01: a task de vídeo segura o socket aberto até o áudio terminar (senão o fim do vídeo encerraria a sessão e cortaria o áudio). PT-1.02: o canal de eventos do servidor tem capacidade 64 — o teste drena continuamente, senão o handshake trava. FT-1.09 (comportamento documentado no teste): a loja é **memória + disco** — apagar `trust.json` com o servidor ligado não afeta o em execução (e o próximo `hello_device` **regrava** o arquivo via `touch`, por isso o teste apaga de novo antes do restart); após restart com o `data_dir` e o arquivo apagado, o device vira `revoked` e o token antigo `bad_token`. IT-1.02/IT-1.06/FT-1.08 e ST-1.03/ST-1.05 confirmados já cobertos (`tests/handshake.rs`, `tests/trust.rs`, `tests/audio_socket.rs`). Medições: PT-1.01 = 45 939 MiB/s (release); PT-1.02 = p95 5,88 ms (n = 500, 0 erros; execuções 6,78 / 9,78 / 5,88 ms); ST-1.06 = `auto_approve_only_in_debug` + `approval_timeout_denies` verdes em `--release`. `lc-server` ganhou `tokio/signal` em dev-deps (exemplo `soak_server.rs`). `soak.ps1` sai com 1 se ws > +5 MB (após 1 min de aquecimento) ou handles > +5%.
- F1-T11b: 13 flags de falha determinísticas em `faults.rs` (`FaultyWriter` com fragmentação, limite de vazão por tempo e stall único; quadros malformados montados à mão). `tests/faults.rs` cobre os 12 cenários (AT-1.07/1.10/1.11/1.12/1.14, IT-1.03/1.04, FT-1.01/1.03/1.06, slow e drop-ping). `lc-server` ganhou `ServerHandle::active_connections()` (observabilidade; remove handles terminados) para o IT-1.03. O IT de fragmentação usa `pattern43.h264` (128 KB) para não estourar o prazo de 5 s com `--fragment 1`; os totais do servidor são comparados com ±2%. `MockConfig`/`Summary`/`MockError` foram para `config.rs` e `session` os reexporta (limite de ~300 linhas; `session.rs` ficou com 280).
- F1-T11a: `mock-sender` (lib + bin) com parser da URI (16 vetores), `--state-file` para reconexão via `device`, vídeo Annex-B com flags/PTS e áudio seno em blocos de 10 ms. Pacing com `interval` + `MissedTickBehavior::Burst` em vez de `Delay` (desvio justificado): a granularidade do timer do Windows (~15,6 ms) transformava 33,3 ms em ~46,8 ms (~21 fps) e 10 ms em ~15,6 ms (~128 blocos/2 s); com Burst a média fica em ~60 quadros e ~200 blocos por 2 s. `session.rs` passou de 300 linhas, então as tasks de mídia foram extraídas para `media.rs` (precedente: `atomic.rs` no F1-T05). `--seconds` permite execução limitada; sem ele roda até o vídeo acabar ou Ctrl+C.
- F1-T16: auditoria de todos os `tracing::*!` (0 valores sensíveis logados — token/`deviceKey`/`audioTicket` nunca eram impressos; só o `deviceId`, que não é segredo). `redact::short`/`Redacted` disponíveis para uso futuro; spans `conn{conn=<id>}` (instrumenta `run_conn`) e `session{session=<4 chars>…}` (instrumenta `session_loop`, com log "sessão iniciada"). `tests/log_redaction.rs` captura TRACE via `MakeWriter` + `set_default` em runtime `current_thread` e cobre pareamento/áudio/reconexão/token errado/ticket reutilizado.
- F1-T10: keepalive (PING 2 s/PONG, liveness 6 s, EWMA α=0,3) e agregador de stats a 500 ms; trabalho parcial do worker cancelado foi auditado e completado (`ConnShared::new` recebe `KeepaliveTiming {ping_interval, liveness_timeout, stats_interval}` para satisfazer `too_many_arguments`; `f32`/`compute_stats` reordenados em `stats.rs` para o rustdoc). `for_tests` mantém keepalive solto (ping 30 s/liveness 90 s) para não derrubar clientes de teste antigos; testes de keepalive passam valores pequenos explícitos.
- F1-T15: `trust/ratelimit.rs` puro com relógio injetado (`AuthLimiter` + `ConnRateLimiter` + `Limiters` com `Mutex` internos; mapa limitado por `prune` a cada consulta). `ServerCtx.limiters: Arc<Limiters>`; accept loop checa `conn_rate.allow` → `auth.is_blocked` **antes** do permit e de qualquer leitura (recusa = `ConnectionClosed{RateLimited}`, sem `ConnectionOpened`, sem resposta — mesmo padrão do `TooManyUnauth`). Desvios do esboço do cartão: `record_failure` devolve `bool` ("acabou de bloquear") para emitir `AuthBlocked{ip}` na hora certa; `Limiters` não deriva `Clone` (`Mutex` não é `Clone` — o clone é do `Arc`); `trust/mod.rs` registrado (linha a mais que a lista do cartão). `for_tests`: `conn_rate_max = 1_000` (solto) — o `unauth_limit_enforced` do `listener.rs` abre 21 conexões do mesmo IP; testes novos usam valores explícitos. API tokio `TcpSocket` (1.54.1): `new_v4()` → `bind(&self)` (sync) → `connect(self)` (async) — `TestClient::connect_from` usa isso para o teste de IP distinto via `127.0.0.2` (toda 127/8 é loopback). Decisões: falha durante bloqueio não conta nem estende (bloqueio expira no `block_until` original); tentativas **recusadas** não entram na janela de conexões (a tempestade não entope a si mesma); `revoked` não conta como falha de auth (só `bad_token`/`BadKey`); `#[allow(dead_code)]` em `mod common` de `tests/handshake.rs` (mesmo padrão de `approval.rs`/`usb_window.rs`). `tests/ratelimit.rs` 296 linhas; `ratelimit.rs` 197.
- F1-T07: `trust/usb_window.rs` (módulo puro, relógio injetado; limite inclusivo `now <= open_until`). Ripples: `ServerCtx.usb_window: Arc<Mutex<UsbWindow>>`; `ServerHandle` guarda `events_tx` (clone) para `open_usb_window()` emitir `UsbWindowOpened`; eventos `UsbWindowOpened`/`UsbWindowClosed{used}` em `events.rs`. Decisão de design: expiração **preguiçosa** (`expire_if_stale` no próximo handshake) — sem timer em segundo plano; `UsbWindowClosed{used:false}` só sai se um pareamento chegar depois da expiração. Check da janela **depois** do token (token inválido não consome) e **antes** do `auto_approve` (janela consumida aprova `ApproveAlways` em qualquer build). Clippy: `if_same_then_else` → condição unificada `usb_consumed || effective_auto_approve(...)`; `MutexGuard` não pode atravessar `.await` → lock em bloco escopo. `tests/usb_window.rs` 299 linhas (7 testes da tabela).
- F1-T06: stub `decide_approval` removido. `Decision`/`ApprovalRequest`/`ApprovalOutcome` + eventos `ApprovalRequested`/`ApprovalResolved` em `events.rs` (conforme cartão); `trust/approval.rs` novo com `ApprovalManager` (máx. 1 pendente: `begin`/`decide`/`cancel`) e `effective_auto_approve(flag, debug_build)`. Ripples exigidos pela integração: `perform` ganhou parâmetros `events_tx`/`stream`/`buf` (fluxo pendente precisa enviar `PENDING` e ler a conexão para detectar o celular sair) e `conn.rs` chama assim; `write_msg` movido de `conn.rs` para `handshake.rs` como `pub(crate)` (compartilhado). `PENDING{timeoutS}` é calculado de `approval_timeout` (`as_secs().max(1)`: padrão 30 s → 30; teste 300 ms → 1). Timeout com decisão chegada no mesmo instante: `rx.try_recv()` antes de concluir `TimedOut` (decisão vale; race residual minúscula documentada). `conn.rs` 392 linhas e `handshake.rs` 492 (cartão fixa o arquivo; fluxo do `Pair` é o núcleo) · `tests/approval.rs` 354 (9 testes da tabela, precedentes: `tests/handshake.rs` 477).
- F1-T04b: stub de aprovação `decide_approval(auto_approve) → ApproveAlways | Reject` — **deixado para F1-T06** (que troca por `PENDING` + timeout + decisão humana; `ApproveOnce` já está declarada no enum, inatingível até lá). Desvios mínimos do esboço do cartão (todos exigidos pela clippy/testes): `CloseReason::AuthDenied` adicionado (fechamento pós-recusa precisa de motivo); `ServerHandle::trust() -> Arc<Mutex<TrustStore>>` (testes e UI de F1-T06 leem/rotacionam token e `revoke`); `perform(ctx, conn_id, first, peer)` — 1º quadro `AUDIO_HELLO` → `bad_audio_ticket`, demais → `ProtocolError`; `SessionRegistry::claim` retorna `Granted{kicked, kick_rx}`/`Busy` (esboço omitia kick_rx); `Outcome::Accepted(Box<Accepted>)` e struct `ConnShared` (lints `large_enum_variant`/`too_many_arguments`); `for_tests(dir, auto_approve)` ganhou o 2º parâmetro (ripple: `tests/listener.rs` passa `false`). Bug corrigido pelos testes: `claim()` com `guard.take()` descartava a sessão ativa no ramo `Busy` → `as_ref()` + `take()` só na substituição. Arquivos acima de ~300 linhas: `conn.rs` 399, `tests/handshake.rs` 477 (cartão fixa esses arquivos; teste único exigido).
- F1-T09a: `TicketStore` guarda `sessionId → Entry{ticket, issued_at}` (1 ticket por sessão); `redeem` localiza a dona do ticket com `ct_eq` (varre todas as entradas, sem saída antecipada) e **consome** o ticket também quando expirado (sem "renovação" por repetição); `Debug` mostra só contagens (sem tickets nem prefixos). Clippy exigiu `impl Default` (lint `new_without_default`).
- F1-T05: `write_atomic`/`write_atomic_with` ficaram em `src/trust/atomic.rs` (módulo separado) para manter `store.rs` dentro do limite de ~300 linhas; os testes de `tests/trust.rs` estão dentro de `mod trust` para o filtro `cargo test -p lc-server trust` casar com eles. `sha2` 0.11 (API `Sha256::new/update/finalize`) e `getrandom` 0.4 (`fill`) confirmados no fonte do registro antes de usar.
- F1-T04a: o rascunho do cartão sugeria `shutdown(self)`, mas isso consumiria o receiver de eventos e tornaria o teste `shutdown_closes_all` impossível de verificar; mantido `shutdown(&mut self)` (idempotente) para permitir ler os `ConnectionClosed{Shutdown}` após o shutdown. Resto da API do cartão intacto.
- F0-T04 (resolvido em F0-T03): `typescript` do `packages/shared` foi alinhado para `~6.0.3`; a versão 7.0.2 duplicava os tipos do i18next e quebrava o typecheck do mobile.
- F0-T02: shadcn/ui criado manualmente (CLI que geraria Tailwind 4 foi pulada); `baseUrl` removido do tsconfig por TS5101; chave extra i18n `desktop.smoke.close` para o label do X do Dialog.
- F0-T03: `@capacitor/app` estável existe só até 8.1.2; core/cli/android em 8.5.3, peer >=8 (compatível).
- F0-T03: i18n passou a usar `I18nextProvider` em desktop e mobile; sem isso o APK mostrava as chaves literais.
- F0-T03: APK instalado e validado por screenshot no Samsung SM-G781B; Moto G5 (API 27) ainda não testado.
- F0-T14: `new-report.ps1` lê o template como UTF-8 e grava sem BOM; sem isso o PowerShell 5.1 corrompia acentos e o separador `·`.
- F0-T05: `desktop/src-tauri` mantém edition 2024 (template Tauri); demais crates em 2021.
- F0-T05: Cargo avisa que `[profile.release]` em `desktop/src-tauri/Cargo.toml` é ignorado (pacote não-raiz); mover para o `Cargo.toml` raiz quando formos gerar release (Fase 4).
- F0-T13: script `make-fixtures.ps1` checa `$LASTEXITCODE` após cada ffmpeg/cargo; temporários ficam em `fixtures/` (PS 5.1 `Join-Path` com caminho absoluto quebra). Fixtures geradas e idempotentes (hashes conferidos).
- F0-T09: Vitest 5 + jsdom 30; UT-0.04 usa identificador minúsculo porque `no-literal-string` ignora `const` MAIÚSCULAS (o exemplo do cartão não era reportado).
- F0-T11: Kotlin 2.4.21 + coroutines-test 1.11.0. O `androidTest` do template esperava `com.getcapacitor.app`; corrigido para `com.localcamera.app`.
- F0-T11: `gradlew connectedDebugAndroidTest` (raiz) falha em `:capacitor-cordova-android-plugins:checkDebugAndroidTestDuplicateClasses` (stdlib 1.8.22 + jdk7/jdk8 1.6.21, módulo gerado). Usar `gradlew :app:connectedDebugAndroidTest`; 2 testes passaram no Samsung.
- F0-T06: `ci.ps1 -Fast` = 17,5 s; completo = 22,9 s; hook `pre-push` ativo. FT-0.01 a FT-0.05 verificados e revertidos (FT-0.01 parou no lint antes do typecheck; typecheck direto também falha).
- F0-S3: veredito **CameraX (única) + Camera2 (dual)**. T1/T2 1080p30 estáveis 10 min (29,7 fps, jitter <1); dual T4 = back+front/ultrawide a 1080p cada, 10 min. **60 fps não oferecido** (`offers60=false`; Camera2 em alvo 60 trava, 0 frames). Armadilha: CameraX `setTargetResolution` cai em 1088x1088 → usar `ResolutionSelector` 16:9. Encoder `OMX.qcom.video.encoder.avc`, sem `KEY_LOW_LATENCY`, 1º keyframe 0,7–1,1 s. **Pendente (humano):** testes de interrupção (tela apagando → exige serviço em primeiro plano na Fase 2).
- F0-S5: parte do agente pronta (protótipo Tauri+React em `spikes/s5/`, medições iniciais 60 s: A WebCodecs 30 fps/1,13 MB/s/11 ms/53 MB; B JPEG 30 fps/0,66 MB/s/8–9 ms/36 MB; ambos memória flat). Coleta automatizada por `spikes/s5/run-s5.ps1` (CPU/mem/GPU por processo a 1 s; `-Dpi` é rótulo; CSV→`docs/spikes/tmp/`). **Status (2026-10-29): CONCLUÍDO** — grupo 100% medido (~8,5 min de dados por modo, app encerrou por timer próprio; drops=0 nos dois): A 29,99 fps / 1,133 MB/s / lat 12/20 ms / main 28,3% + árvore 54% / mem 29→55 MB flat (GPU `n/a` na execução longa, 14,1% no smoke); B 29,77 fps / 0,653 MB/s / lat 10/14 ms / main 16,7% + árvore 48,2% / mem 29→37 MB flat / GPU 1,1% → **veredito B** (ADR 0005). Seção "janela minimizada" do modelo do cartão ficou pendente (fora do critério de pronto; follow-up). CSVs/JSON em `docs/spikes/tmp/s5-{A,B}-100.*` + `spikes/s5/auto-{A,B}-100.json`; custo de decodificação H.264 medido por VELOCIDADE (contabilidade de CPU por processo desconfiável nesta máquina — `Process.CPU`/`GetProcessTimes` inconsistentes p/ ffmpeg): sw 682 fps / sw 1-thread 205 fps / d3d11va 233 fps → 30 fps de preview custa ~4,4% de 1 núcleo (sw multi) / ~14,6% (sw 1 núcleo) / ~0 (GPU) — `spikes/s5/measure-decode.ps1` + `docs/spikes/tmp/s5-decode.json`. Fixture `hd30.h264` = 300 frames @ 25 fps (12 s, não 30 fps como o nome sugere). Grupos 150/200% bloqueados por sign-out (ver Bloqueios).
- F0-T10: e2e **não incluído no `ci.ps1`** — incluir na Fase 2, junto com o catálogo real.
- F0-T10: `snapshotDir` ficou descontinuada no Playwright (1.64); usado `snapshotPathTemplate: "e2e/__snapshots__/{projectName}/{arg}{ext}"` (mesmo diretório do cartão). No Playwright 1.64 o `testInfo` é o 2º argumento do teste (não fixture), e `devices["iPhone 13"]` traz `webkit` - projeto mobile fixado em `chromium`.
- F0-S1: câmera de teste removida; depois do reboot, o sample enumerou o device mas `Remove` retornou `MF_E_NOT_FOUND`; câmera sumiu das Configurações e os arquivos do spike foram removidos.
- F0-S2 (concluído): pipe nomeado é o canal Frame Server ↔ app (veredito aceito, ADR 0003). Medições: 1500 s = 41.978 quadros, 0 perdas, 87,0 MB/s, lat p50 1,87 ms / p95 2,95 ms; CPU 60 s (câmera aberta): app 3,54% / FrameServer 100,78%; 20× abrir/fechar = 21 reconexões OK; segurança 2º usuário = NEGADO 5 (ACCESS_DENIED); shm Global = NEGADO (0x80070005), shm Local = 93,3 MB/s. **Cleanup de admin CONCLUÍDO (2026-10-29)** via `spikes/s2/cleanup-elevado.ps1` em PowerShell elevado (CLSID + `C:\Program Files\LocalCameraSpike` + chave HKLM + `C:\ProgramData\LocalCameraElevated` + tarefa + usuário `LocalCamSpikeTest`; verificação final "All clean.").

---

## Fase 0 — Fundação, testes e spikes  → `docs/phases/fase-0-fundacao-e-spikes.md`
Ordem de execução (respeite as dependências):

| St | ID | Título | Exec | Dep. | Cartão |
|---|---|---|---|---|---|
| [x] | F0-T01 | Monorepo pnpm, ESLint, Prettier, tsconfig | A | — | `docs/tasks/fase-0/F0-T01.md` |
| [x] | F0-T07 | Estrutura de docs, ADR 0001, scripts de relatório | A | T01 | `docs/tasks/fase-0/F0-T07.md` |
| [x] | F0-T04 | Pacote `@localcamera/shared` (cn, preset, tokens, i18n) | A | T01 | `docs/tasks/fase-0/F0-T04.md` |
| [x] | F0-T02 | App desktop: Tauri 2 + React 19 + Tailwind 3.4 + shadcn | A | T04 | `docs/tasks/fase-0/F0-T02.md` |
| [x] | F0-T03 | App mobile: Capacitor + React 19 + Tailwind 3.4 + shadcn | A+H | T04 | `docs/tasks/fase-0/F0-T03.md` |
| [x] | F0-T05 | Workspace Rust (lc-protocol, mock-sender, toolchain) | A | T01 | `docs/tasks/fase-0/F0-T05.md` |
| [x] | F0-T09 | Infra de testes TypeScript (Vitest, RTL, testes UT-0.01–04) | A | T04 | `docs/tasks/fase-0/F0-T09.md` |
| [x] | F0-T12 | Infra de testes Rust (`lc-testkit`, proptest, tokio) | A | T05 | `docs/tasks/fase-0/F0-T12.md` |
| [x] | F0-T11 | Infra de testes Kotlin/Android (JUnit, coroutines-test) | A+H | T03 | `docs/tasks/fase-0/F0-T11.md` |
| [x] | F0-T06 | CI local (`scripts/ci.ps1`) e hook de `pre-push` | A | T09, T12 | `docs/tasks/fase-0/F0-T06.md` |
| [x] | F0-T13 | Fixtures: `make-fixtures.ps1` e `fixture-tool` | A | T12 | `docs/tasks/fase-0/F0-T13.md` |
| [x] | F0-T14 | Relatórios de teste (`new-report.ps1`) | A | T07 | `docs/tasks/fase-0/F0-T14.md` |
| [x] | F0-T10 | (opcional) Playwright para o catálogo de componentes | A | T02, T03 | `docs/tasks/fase-0/F0-T10.md` |
| [x] | F0-S1 | Spike S1 — câmera virtual × NVIDIA Broadcast | A+H | T01 | `docs/tasks/fase-0/F0-S1.md` |
| [x] | F0-S2 | Spike S2 — canal Frame Server ↔ app (pipe nomeado, veredito aceito; ADR 0003; cleanup de admin concluído em 2026-10-29) | A+H | S1 | `docs/tasks/fase-0/F0-S2.md` |
| [x] | F0-S3 | Spike S3 — CameraX × Camera2 (e câmeras simultâneas) | A+H | T03 | `docs/tasks/fase-0/F0-S3.md` |
| [x] | F0-S5 | Spike S5 — estratégia de preview (veredito **B** — JPEG do Rust; ADR 0005; 150/200% pulados por decisão do usuário) | A+H | T02, T13 | `docs/tasks/fase-0/F0-S5.md` |
| [x] | F0-GATE | Portão da Fase 0 (relatório `docs/tests/fase-0.md` APROVADA; CI completo + `-Fast` verdes; AT-0.01 clone limpo verde; AT-0.04 APK via ADB; AT-0.06 diferido p/ Fase 9; D-0 = timeout UT-0.04 corrigido; cleanup S2 concluído; vereditos confirmados pelo usuário) | A+H | todas | `docs/tasks/fase-0/F0-GATE.md` |

## Fase 1 — Protocolo e rede  → `docs/phases/fase-1-protocolo-e-rede.md`
Ordem de execução (respeite as dependências). A lógica de rede fica na crate `crates/lc-server` (sem Tauri).

| St | ID | Título | Exec | Dep. | Cartão |
|---|---|---|---|---|---|
| [x] | F1-T01 | Vetores de teste do protocolo (`protocol-vectors.json`) | A | F0-GATE | `docs/tasks/fase-1/F1-T01.md` |
| [x] | F1-T02a | `lc-protocol`: cabeçalho e `FrameReader` | A | T01 | `docs/tasks/fase-1/F1-T02a.md` |
| [x] | F1-T02b | `lc-protocol`: mensagens JSON (serde) e `Message` | A | T02a | `docs/tasks/fase-1/F1-T02b.md` |
| [x] | F1-T03 | Tipos/validadores TS + URI de pareamento (+ vetores de URI) | A | T01 | `docs/tasks/fase-1/F1-T03.md` |
| [x] | F1-T04a | `lc-server`: listener, limites e laço de leitura | A | T02b | `docs/tasks/fase-1/F1-T04a.md` |
| [x] | F1-T05 | Confiança: chaves, hashes, armazenamento atômico | A | T04a | `docs/tasks/fase-1/F1-T05.md` |
| [x] | F1-T09a | Tickets de áudio (`TicketStore`, lógica pura) | A | T05 | `docs/tasks/fase-1/F1-T09a.md` |
| [x] | F1-T04b | Handshake, autenticação, versão e sessão única | A | T04a, T05, T09a | `docs/tasks/fase-1/F1-T04b.md` |
| [x] | F1-T06 | Fluxo de aprovação (`PENDING`, timeout, decisão) | A | T04b | `docs/tasks/fase-1/F1-T06.md` |
| [x] | F1-T07 | Janela de aprovação USB (15 s, localhost) | A | T06 | `docs/tasks/fase-1/F1-T07.md` |
| [x] | F1-T15 | Limitação de falhas de auth e de taxa de conexões | A | T04b | `docs/tasks/fase-1/F1-T15.md` |
| [x] | F1-T09b | Socket de áudio (`AUDIO_HELLO`, contadores) | A | T04b, T09a | `docs/tasks/fase-1/F1-T09b.md` |
| [x] | F1-T10 | Keepalive, RTT e agregador de estatísticas (500 ms) | A | T09b | `docs/tasks/fase-1/F1-T10.md` |
| [x] | F1-T16 | Logs com redação de segredos | A | T04b | `docs/tasks/fase-1/F1-T16.md` |
| [x] | F1-T11a | `mock-sender`: pareamento, reconexão, vídeo/áudio | A | T09b, F0-T13 | `docs/tasks/fase-1/F1-T11a.md` |
| [x] | F1-T11b | `mock-sender`: flags de falha + AT/FT automatizados | A | T11a, T10 | `docs/tasks/fase-1/F1-T11b.md` |
| [x] | F1-T13 | Integração, desempenho e *soak* (`integration.rs`: IT-1.01, AT-1.15, FT-1.09, PT-1.02; `perf.rs`: PT-1.01; `soak.ps1` + `soak_server.rs`; `docs/perf/fase-1.md` — PT-1.01 45 939 MiB/s, PT-1.02 p95 5,88 ms, ST-1.06 verde em release; **soak LT-1.01 agendado**, ver *Bloqueios*) | A+H | T11b, T15, T16, T07 | `docs/tasks/fase-1/F1-T13.md` |
| [x] | F1-GATE | Portão da Fase 1 — **APROVADA** (relatório `docs/tests/fase-1.md`; CI completo + vetores + perf verdes; soak LT-1.01 aprovado; protocolo v1.0; cobertura 70/70 IDs; ST-1.05 ferramentas "não executado" por falta de aprovação) | A | todas | `docs/tasks/fase-1/F1-GATE.md` |

## Fase 2 — Mobile: transmissão mínima  → `docs/phases/fase-2-mobile-transmissao-minima.md`
Ordem de execução (respeite as dependências). Kotlin em `mobile/android/app/src/main/java/com/localcamera/app/`.

| St | ID | Título | Exec | Dep. | Cartão |
|---|---|---|---|---|---|
| [ ] | F2-T01 | Capacitor e deep link (`localcamera://connect`) | A+H | F1-GATE | `docs/tasks/fase-2/F2-T01.md` |
| [ ] | F2-T03a | Tokens e componentes próprios | A | T01 | `docs/tasks/fase-2/F2-T03a.md` |
| [ ] | F2-T03b | Textos i18n (JSON + `strings.xml`) | A | T01 | `docs/tasks/fase-2/F2-T03b.md` |
| [ ] | F2-T03 | Telas e roteamento | A | T03a, T03b | `docs/tasks/fase-2/F2-T03.md` |
| [ ] | F2-T03c | Catálogo (`?catalog`) | A | T03 | `docs/tasks/fase-2/F2-T03c.md` |
| [ ] | F2-T04 | `ProtocolCodec.kt` (vetores 100%) | A | F1-GATE | `docs/tasks/fase-2/F2-T04.md` |
| [ ] | F2-T05a | Parser da URI `localcamera://connect` | A | F1-GATE | `docs/tasks/fase-2/F2-T05a.md` |
| [ ] | F2-T05 | `PairingStore.kt` (Keystore, fora do backup) | A+H | T01 | `docs/tasks/fase-2/F2-T05.md` |
| [ ] | F2-T06 | `CapabilitiesProbe` | A+H | T01 | `docs/tasks/fase-2/F2-T06.md` |
| [ ] | F2-T07 | `CameraBackend` (CameraX, veredito S3) | A+H | T06 | `docs/tasks/fase-2/F2-T07.md` |
| [ ] | F2-T08 | `VideoEncoder.kt` (MediaCodec H.264 CBR) | A+H | T07 | `docs/tasks/fase-2/F2-T08.md` |
| [ ] | F2-T09 | `PrimaryConnection.kt` (hosts paralelos, backoff) | A | T04, T05 | `docs/tasks/fase-2/F2-T09.md` |
| [ ] | F2-T11 | `StateReducer.kt` | A | T04 | `docs/tasks/fase-2/F2-T11.md` |
| [ ] | F2-T10 | `SendQueue.kt` (2 faixas, descarte por tempo) | A | T09 | `docs/tasks/fase-2/F2-T10.md` |
| [ ] | F2-T16 | Rede do Android (Wi-Fi sem internet vs. móveis) | A+H | T09 | `docs/tasks/fase-2/F2-T16.md` |
| [ ] | F2-T12 | `StreamingService` (foreground) | A+H | T08, T09 | `docs/tasks/fase-2/F2-T12.md` |
| [ ] | F2-T13 | `LocalCameraPlugin.kt` | A+H | T12 | `docs/tasks/fase-2/F2-T13.md` |
| [ ] | F2-T14 | Testes (agrupador: UT/IT/AT/FT/PT/ST) | A+H | T03, T03c, T04, T05, T06, T08, T09, T10, T11, T12, T13, T16 | `docs/tasks/fase-2/F2-T14.md` |
| [ ] | F2-T15 | Relatório e matriz de aparelhos (`docs/devices.md`) | A+H | T14 | `docs/tasks/fase-2/F2-T15.md` |
| [ ] | F2-GATE | Portão da Fase 2 (relatório + regressão RS-S/AT-1.xx) | A | todas | `docs/tasks/fase-2/F2-GATE.md` |

## Fase 3 — Desktop: UI, decodificação e preview  → `docs/phases/fase-3-desktop-ui-e-preview.md`
**Cartões: (gerar)** — depende do veredito do S5. Tarefas previstas:
`F3-T01` janela e titlebar · `F3-T02` componentes e animações · `F3-T03` i18n · `F3-T13` comandos/eventos Tauri · `F3-T14a` store · `F3-T04` estado vazio e aba Conexão · `F3-T05` `ApprovalDialog` · `F3-T06` confiáveis · `F3-T07` decodificador · `F3-T08` composição + `composition.md` · `F3-T09` preview · `F3-T10` regras de desempenho · `F3-T11` chips e barra de ferramentas · `F3-T14a2` painel lateral · `F3-T12` `HealthBar` · `F3-T14c` catálogo · `F3-T17` instrumentação de latência · `F3-T14` testes · `F3-GATE`

## Fase 4 — Câmera virtual  → `docs/phases/fase-4-camera-virtual.md`
**Cartões: (gerar)** — depende dos vereditos S1 e S2. Tarefas previstas:
`F4-T01` DLL · `F4-T02` cliente do canal · `F4-T03` placeholder · `F4-T04` módulo `vcam/` · `F4-T05` indicador · `F4-T06` `dev-setup.ps1` · `F4-T07` assistente · `F4-T08` plano B DirectShow (se S1 mandar) · `F4-T09` testes longos · `F4-T10` README · `F4-T11` métricas · `F4-T12` `vcam-probe` · `F4-GATE` (**tag `v0.1.0`**)

## Fase 5 — USB e descoberta  → `docs/phases/fase-5-usb-e-descoberta.md`
**Cartões: (gerar).** `F5-T01` localizar adb · `F5-T02` rastrear aparelhos · `F5-T03` fluxo Conectar · `F5-T04` UI · `F5-T05` ciclo de vida · `F5-T11` auto-conectar · `F5-T06` anúncio mDNS · `F5-T07` firewall · `F5-T08` `Discovery.kt` · `F5-T09` cadeia de reconexão · `F5-T10` PCs conhecidos · `F5-T12` `fake-adb` · `F5-T13` testes · `F5-GATE`

## Fase 6 — Controle bidirecional  → `docs/phases/fase-6-controle-bidirecional.md`
**Cartões: (gerar).** `F6-T01` reducer · `F6-T02` reconfiguração · `F6-T03` otimismo/reconciliação · `F6-T04` espelho/rotação · `F6-T05` aba Câmera · `F6-T06` tela Transmitindo · `F6-T07` STATS + ABR · `F6-T08` keyframe · `F6-T09` indicadores · `F6-T10` persistência · `F6-T11` `--emulate-phone` · `F6-T12` testes · `F6-GATE` (**tag `v0.2.0`**)

## Fase 7 — Câmera dupla  → `docs/phases/fase-7-camera-dupla.md`
**Cartões: (gerar)** — depende do S3. `F7-T01…T17` · `F7-GATE` (**tag `v0.3.0`**)

## Fase 8 — Áudio  → `docs/phases/fase-8-audio.md`
**Cartões: (gerar)** — começa pelo spike S4. `F8-S4a/b/c` · `F8-T01…T15` · `F8-GATE` (**tag `v0.4.0`**)

## Fase 9 — Polimento e instalação  → `docs/phases/fase-9-polimento-e-instalacao.md`
**Cartões: (gerar).** `F9-T01…T17` · `F9-GATE` (**tag `v1.0.0`**)
