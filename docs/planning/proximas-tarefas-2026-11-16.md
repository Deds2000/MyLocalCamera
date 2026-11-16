# Próximas tarefas — plano de execução (2026-11-16)

**Estado: plano consolidado; execução condicionada.** Planejamento solicitado pelo usuário enquanto a F3-T06 é retomada, com detalhamento de 13 cartões seguintes. Este documento não aprova testes, não substitui cartões e não libera dependências bloqueadas.

## Regras de execução

- Fonte de ordem e estado: `PROGRESS.md`; executar uma tarefa por vez, escolhendo a primeira pendente com dependências concluídas após fechar a tarefa ativa.
- Capacidade autorizada: até 10 Luna. Nesta rodada, dois trabalham na F3-T06 (UI e harness em arquivos separados) e oito fazem análises somente leitura. Não há autorização para implementar oito cartões simultaneamente.
- O gerente integra e revisa os resultados, verifica os critérios e faz commits seletivos. Nenhuma alteração anterior de F3-T05, F3-T07 ou F2-FIX-02 entra incidentalmente em outro commit.
- Antes de testar, comparar critério e versão com a evidência existente. Após mudança localizada, executar somente as verificações afetadas; não repetir soaks ou testes verdes para reformatação de relatório.
- Aparelho/desktop: IA por ADB, UIA e harnesses permitidos. Sem pedir operação, coleta ou inspeção manual ao usuário. Simulação não equivalente não aprova critério.
- Segredos nunca inteiros em saída, log ou evidência; CDP de diagnóstico somente em loopback e temporário. Cleanup limitado a processos e dados criados pela tarefa.
- Duas falhas no mesmo ponto: parar, registrar erro/tentativas em `PROGRESS.md` e solicitar decisão técnica. Não executar novamente sem mudança de causa.
- Testes acima de 15 minutos: somente se exigidos pelo cartão e sem evidência equivalente, com duração informada antes. Sem driver experimental na máquina principal.
- Novos módulos de código: até aproximadamente 300 linhas; textos UI por chaves pt-BR/en e backend por códigos estáveis. Não criar arquitetura, APIs, flags ou versões por suposição.

## Ordem condicionada ao estado atual

| Ordem de seleção | Tarefa | Condição |
|---|---|---|
| Ativa | F3-T06 | Fechar recuperação de reconsulta e diagnosticar o mock; AT-3.03 somente após correção de causa demonstrada |
| Próxima executável | F2-FIX-04 | F2-FIX-01 concluída; sem depender da medição de latência bloqueada |
| Seguinte executável | F3-T12 | T13, T14a e T02 concluídas; selecionar depois das anteriores na tabela de progresso |
| Depois | R-02, R-03 e R-04 | R-03 só após R-02; se R-02 bloquear por D4, R-04 pode ser a próxima elegível (não depende de R-03) |
| Pipeline bloqueado | F3-T08 → F3-T09 → F3-T10 → F3-T11 → F3-T14a2 | F3-T07 não está concluída; nenhum desses itens é liberado por este plano |
| Fechamento UI/perf | F3-T14c e F3-T17 | Respeitar também F3-T05, F3-T12 e os predecessores específicos |
| Gate Fase 2 | F2-GATE-R | F2-FIX-02 continua bloqueada; FIX-03 e FIX-04 precisam estar concluídas |

Recalcular a seleção após cada conclusão/decisão. A tabela acima é uma projeção, não uma reordenação da sequência aprovada. Fase 4 fica fora deste detalhamento.

## Decisões que este plano não resolve

- F2-FIX-02: p95 acima de 80 ms em seis execuções; não alterar limite nem aceitar desvio sem decisão.
- F3-T07: IT-3.04 bloqueado por `DXGI_ERROR_DRIVER_INTERNAL_ERROR`; não repetir GPU sem mudança concreta de causa.
- F3-T05: validação real de Esc pendente; autorização de diagnóstico da F3-T06 não autoriza outro harness de teclado para essa tarefa.
- D3: instalação de `cargo audit`/`cargo deny`; D4: remoção de `LocalCamera-docs.zip`; D6: aceitação de desvios no gate.
- Divergência dos cartões F3-T14 agregador/F3-GATE: busca nominal em `docs/tasks/fase-3/` nesta revisão não encontrou esses dois cartões. Reconciliar o registro/proposta antes de fechar o gate; não criá-los nem presumir critérios nesta rodada.

## Detalhamento por cartão

Os blocos abaixo detalham os cartões, não sua aprovação. Comandos são os prescritos ou já confirmados no pacote; executá-los somente quando a tarefa estiver ativa. Lacunas explicitadas não devem ser preenchidas por suposição.

### F3-T06 — fechar a tarefa ativa

1. Integrar/revisar `TrustedList.tsx`, `trusted.test.tsx`, `ConnectionTab.tsx` e catálogos pt-BR/en; nenhuma edição no contrato de trust ou protocolo.
2. Recuperação: após revogação positiva e reconsulta falha, oferecer **Atualizar lista**; uma consulta por ação, sem revogar novamente, sem remoção otimista, com erro recuperável e foco lógico. Nomes longos devem quebrar no diálogo.
3. Diagnóstico separado do harness: localizar a causa da saída 1 antes da aprovação usando fontes/argumentos sintéticos e categorias de erro sanitizadas. Preservar o JSON histórico BLOCKED.
4. Só após causa corrigida e UI verificada: app debug com CDP loopback temporário e decoder software; mock único com credencial temporária; aprovar permanente por UIA, cancelar a remoção primeiro, confirmar depois, reconectar com o mesmo state-file e observar `revoked`/saída 2 sem reaparecimento.
5. Reutilizar testes anteriores não afetados. Recuperação atual: 9 testes direcionados, paridade i18n 2, typecheck/lint verdes; gerente ainda revisa integração e AT. Capturas somente da seção/diálogo seguros, nunca QR nem outros apps.
6. Pronto: UT-3.22 e AT-3.03 reais cobertos, traduções e lint/typecheck verdes, evidência revisada, cleanup conferido. Commit seletivo: `feat(desktop): dispositivos confiaveis com revogacao (F3-T06)`.

### F2-FIX-04 — reclassificar lacunas de evidência

**Entrada:** FIX-01 concluída; próxima elegível depois da tarefa ativa, mantendo os bloqueios atuais. **Arquivos:** `docs/tests/fase-2.md` (Resultados e Desvios/limitações), depois `PROGRESS.md`.

1. Auditar individualmente as dez linhas: AT-2.06/07/08/09/10, FT-2.02/04/09/14 e PT-2.06. Escopar a contagem à tabela, não às menções históricas no texto.
2. AT-2.06 permanece N/A pela decisão de 2026-11-11, fora do gate. Os outros nove permanecem PENDENTE sem evidência nova: banda, revogação, versão incompatível, QR nativo, troca de rede, chamada, memória, 20 QR inválidos e tempo QR→Transmitindo.
3. Associar referência ou justificativa a cada linha; preservar FT-2.10/11 e F2-T16 fora do gate. Substituir a ideia de teste manual por automação fiel ou bloqueio explícito, sem inventar aprovação.
4. Matriz registra base `18d9c98`, APK debug 1.0 e protocolo 1.0; confirmar a versão para cada evidência reutilizada. FIX-01 cobre queda/reconexão, não troca de Wi-Fi (FT-2.02).
5. Verificação do cartão é auditoria textual por busca/contagem dos dez IDs e revisão das referências; não há comando literal nem teste de aparelho a repetir.
6. Pronto: dez IDs classificados separadamente, sem promoção de pendência a verde. Commit: `docs(testes): reclassificar gaps da Fase 2 (F2-FIX-04)`.

### F2-GATE-R — relatório e decisão de gate

**Bloqueado:** FIX-02 não está concluída; FIX-03 está concluída; FIX-04 ainda não. **Entrada:** matriz e evidência FIX-01…04, sem repetir medições equivalentes.

1. Após liberar dependências, reunir referências por critério/versão e escrever novo relatório no local coerente com os relatórios existentes, preservando o histórico.
2. Separar aprovado, parcial, pendente, desvio aceito e fora de escopo. Resultado p95 acima de 80 ms não vira conformidade; distinguir o desvio histórico PT-2.02 da FIX-02 ainda bloqueada.
3. Auditar rastreabilidade e limitações, incluindo regressão de segurança incompleta e reconciliação F3-GATE/F3-T14. Ausência de artefato ou de automação equivalente deixa o critério pendente.
4. Pronto do relatório não significa fase aprovada: apresentar ao usuário para aprovar, aprovar com desvios ou manter pendente. Até essa decisão, **Fase 2 NÃO APROVADA**.
5. Verificação: cada critério com referência verificável, nenhuma exceção convertida em aprovação. Commit: `docs(testes): reavaliar gate da Fase 2 (F2-GATE-R)`.

### F3-T08 — composição NV12 (condicionada a T07)

**Arquivos:** `docs/composition.md`, `desktop/src-tauri/src/compose/{mod,nv12}.rs`, `desktop/src-tauri/tests/compose.rs`.

1. Documentar a especificação única: rotação `(hint + usuário) % 360` → espelho horizontal → contain/cover → letterbox preto → NV12 fixo 1920×1080. Fixar arredondamento/pixel extra e exemplos 16:9, 4:3 e retrato; preto conforme cartão Y=0, UV=128/128.
2. Implementar API CPU `Fit`, região e `ComposeInfo`; helpers de planos/stride em módulos curtos. Não implementar GPU nem trocar o tamanho de saída. SIMD só se benchmark justificar.
3. Teste gera quatro quadrantes/marcadores; referência independente calcula golden/hash e pixels-chave, sem reutilizar algoritmo de produção.
4. UT-3.01/02/03/04 cobrem rotações, soma do hint, ordem do espelho, fits, dimensões ímpares e letterbox simétrico. Testes de decoder T07 não substituem esses critérios.
5. Verificações: `cargo test -p desktop --test compose`; `cargo clippy -p desktop`. Não iniciar enquanto T07 estiver parcial nem transformar CPU verde em aprovação GPU.
6. Pronto: especificação e quatro UTs verdes, sem warnings novos. Commit: `feat(desktop): compose 1920x1080 NV12 + docs/composition.md (F3-T08)`.

### F3-T09 — JPEG e canvas imperativo (condicionada a T08/T14a)

**Arquivos:** `desktop/src-tauri/src/preview.rs`, `desktop/src-tauri/tests/preview.rs`, `desktop/src/lib/preview-player{,.test}.ts`, `desktop/src/features/preview/{PreviewCanvas,PreviewArea}.tsx`.

1. Consumir NV12 composto; downscale documentado para 960×540, JPEG e limite de 30 fps/intervalo mínimo do cartão. Descartar trabalho atrasado sem bloquear pump. Expor pause/resume por sessão.
2. Confirmar APIs nas versões instaladas Tauri/Rust/JS antes de medir transporte. Estratégia B já está escolhida; transporte compatível e média JPEG são medidos e registrados em Observações, sem presumir binário zero-copy.
3. Player com fila ~4, descarte do mais antigo, draw na chegada por IPC (sem timer), `ImageBitmap.close()`, reuso de buffers, fps/timestamps. Nenhum quadro no store/React.
4. Canvas usa ref e tamanho físico CSS×DPR; pausa esvazia fila e preserva último quadro, retomada no próximo frame. Integração visual completa de estados fica para T11.
5. UT-3.09 (fila); IT-3.01 (ordem/continuidade); FT-3.03 (`sps-change` sem apagar); FT-3.06 (consumidor lento); AT-3.04 (~30 fps); AT-3.15 (DPI100%). DPI150/200 sem medição fica desvio/pendência registrada, não aprovação automática nem alteração global do Windows.
6. Verificações: `cargo test -p desktop --test preview`; `pnpm --filter desktop test -- preview-player features/preview`; `pnpm --filter desktop typecheck`. Lint canônico do pacote confirmado: `pnpm --filter desktop lint`.
7. S5 é baseline, não aprovação do produto: 29,77 fps, 0 drops, p95 14 ms, 0,653 MB/s; `Channel<Vec<u8>>` chegou como array JSON e pode ampliar payload 3–5×. Medir JPEG próximo ao alvo ~22 KB/frame.
8. Pronto: testes/AT cobertos ou exceções registradas para decisão, transporte/tamanho medidos, pausa sem limpar quadro. Commit: `feat(desktop): preview estrategia B — jpeg do rust + canvas imperativo (F3-T09)`.

### F3-T10 — desempenho, pausa e renderizações (condicionada a T09)

**Estimativa do cartão:** ~2,5 h. **Arquivos:** `hooks/useVisibility.ts`, `src-tauri/src/preview.rs`, `app/Shell.tsx`, `features/preview/perf.test.ts`, `scripts/soak-f3-cpu.ps1`.

1. Conferir stats Rust→pump→slice, agregador existente `crates/lc-server/src/stats.rs` e seus testes/evidências; não reescrever nem repetir UT-3.10 se já coberto na mesma versão/critério. Limite normativo **≤2 Hz**: o passo “≤2 eventos/500 ms” é inconsistente e não autoriza 4 Hz.
2. Ligar `visibilitychange`/minimização a pause/resume de backend e player, mantendo canvas e sem afetar futura câmera virtual. Confirmar eventos/APIs Tauri instalados; provar retomada <200 ms com stub de transporte.
3. Profiler por 5 s de mock30fps: nenhum render por quadro, stats≤2 Hz, PT-3.05≤4 renders/s e seletores estreitos. Não criar medidor de áudio da Fase 8.
4. AT-3.06: minimizar por 1 min via UIA, CPU antes/durante/depois (amostras cada 5 s), restaurar e evidenciar retomada. LT-3.02: 100 ciclos automatizados, WS/handles a cada 10, sem vazamento; duração efetiva depende do harness e deve ser anunciada se >15 min.
5. Verificações do cartão: `pnpm --filter desktop test -- features/preview hooks`; `cargo test -p desktop`; `pnpm --filter desktop typecheck`. A suíte Rust inclui critério GPU bloqueado no estado atual: não dispará-la agora nem ocultar esse teste; resolver dependências/decisão antes.
6. Pronto: UT-3.10, AT-3.06/08, PT-3.05, LT-3.02, retomada e queda de CPU comprovados. Commit: `feat(desktop): regras de desempenho 2Hz + pausa ao minimizar (F3-T10)`.

### F3-T11 — estados, chips, toolbar e teclado (condicionada a T10)

**Estimativa:** ~3 h. **Arquivos:** `features/preview/PreviewArea.tsx`, novos `{PreviewOverlays,Toolbar}.tsx`, `hooks/useShortcuts.ts`, testes `{states,shortcuts,toolbar}.test.tsx`; store/ponte de apresentação só no mínimo permitido pelo cartão.

1. Switch dos estados empty/connecting/pending/streaming/reconnecting/error/paused. Reconexão conserva canvas + escurecimento50% sem blur; erro é código→i18n e ação existente, não frase backend nem nova reconexão.
2. Chips isolados no slice stats≤2 Hz: live/reconnecting, resolução/fps, bitrate, RTT warn>80 ms/danger>200 ms e perdidos10s só se>0; fonte mono/tabular e sinal além da cor.
3. Toolbar com seis ações/tooltips; somente espelho, giro90 e tela cheia do preview funcionais. Câmera/PiP/mudo desabilitados com motivo/fase; nada de SET ao celular, áudio ou duas webcams.
4. M/R/F/F11/Esc e Shift+M conforme mapa; ignorar teclas únicas em input/textarea/select/contenteditable e ações desabilitadas. Ctrl+,/Ctrl+1…4 pertencem à T14a2. Fullscreen ocupa a janela, não muda fullscreen do sistema; barra esconde após3s só nesse modo e volta por mouse/foco.
5. Slice UI→compose deve produzir efeito local imediato; confirmar ponte existente e, se faltar, usar somente `set_presentation` mínimo permitido pelo cartão, documentando contrato. Toast de recuperação: um por rajada, sem spam.
6. UT-3.20/24; FT-3.04 com `--kill-after`; AT-3.07 compara pixels do canvas à referência golden T08; AT-3.14 usa teclado UIA. Falha de teclado não autoriza substituir por chamada direta a ação nem reaproveitar Esc ainda bloqueado da T05.
7. Verificações: `pnpm --filter desktop test -- features/preview`; `pnpm --filter desktop typecheck`; `pnpm --filter desktop lint`. Pronto exige IDs, thresholds/cadência, disabled tooltips e toast cobertos. Commit: `feat(desktop): estados, chips e toolbar do preview (F3-T11)`.

### F3-T12 — HealthBar (elegível após anteriores executáveis)

**Arquivos:** `features/health/{HealthBar,HealthPopover}.tsx`, `health.test.tsx`, `src-tauri/tests/health.rs`, `app/Shell.tsx`, `src-tauri/src/{commands,events_pump,lib}.rs`; ponte/store/i18n somente no contrato necessário.

1. Reutilizar `HealthInfo.server` (listening/altPort/failed, port/effectivePort), slice `health` e carga inicial em `useTauriEvents`. Inspecionar `stubs` placeholder; não inventar novo esquema só para quatro indicadores fixos.
2. Confirmar API/ciclo de vida `lc_server`/`AppState`; implementar `restart_server` mínimo e evento `health://changed` apenas quando estado/porta muda, não a cada poll. Confirmar payload compatível com HealthInfo antes de editar contrato; ausência de suporte exige decisão técnica, não simulação.
3. Substituir footer32px vazio por cinco indicadores: Servidor dinâmico + Descoberta/Câmera virtual/Microfone virtual/ADB “Em breve”, muted sem ação. Servidor mostra porta efetiva: listening ok, altPort warn, failed danger + Tentar novamente.
4. Popover acessível por teclado, foco visível, loading/erro traduzidos, estado não só por cor. Abrir Ajustes fica desabilitado com tooltip enquanto T14a2 não existir, como o cartão permite.
5. FT-3.08: ocupar48650 com listener de teste próprio, iniciar servidor e comprovar warn/porta no rodapé + mesma porta no QR URI (sem imprimir URI). Um teste Rust isolado não aprova a parte UI integrada.
6. Verificações: `pnpm --filter desktop test -- features/health`; `cargo test -p desktop --test health`; `pnpm --filter desktop typecheck`; lint já confirmado `pnpm --filter desktop lint`.
7. Smoke antigo get_health não cobre restart/evento/FT-3.08. Pronto: cinco indicadores, restart/evento testados, FT-3.08 e checks verdes. Commit: `feat(desktop): HealthBar com servidor funcional (F3-T12)`.

### F3-T14a2 — quatro abas do painel (condicionada a T04/T11/T14b)

**Estimativa:** ~3 h + validação. **Arquivos:** `components/SidePanel.tsx`, `app/Shell.tsx`, `features/{camera/CameraTab,audio/AudioTab,settings/SettingsTab}.tsx` e respectivos testes; catálogos/copy e Rust/ponte só se exigidos pelo contrato do cartão.

1. Confirmar `useShortcuts` entregue por T11 antes de editar; hoje ausente não autoriza criar API concorrente. SidePanel usa Tabs/ScrollArea/animação existentes e mantém largura de `app/layout.ts`, persistindo aba por `ui.setActiveTab`→settings.
2. Câmera reutiliza SettingRow/SegmentedControl: espelho/rotação locais no slice UI; fonte, dual/PiP, resolução/fps/bitrate desabilitados com motivo/fase. Áudio usa controles inativos e LevelMeter zero, sem simular microfone.
3. Ajustes: idioma pelo fluxo real do slice+i18n; nível de log somente após confirmar subscriber/API instalada (a revisão atual não encontrou subscriber desktop nem `tracing-subscriber`). Abrir logs pelo opener existente, verificando API/permissões/escopo seguro; não abrir conteúdo alheio para testar. Exportar e demais futuros ficam desabilitados conforme cartão.
4. Abaixo de1100px, abas com ícone+nome acessível+tooltip, seleção/navegação/foco corretos, motivos de disabled alcançáveis por teclado e reduced motion. Completar somente chaves faltantes em pt-BR/en/copy.
5. Testes de abas/persistência, toggles/rotação, motivos inativos, idioma imediato e falhas log/opener por contrato mock. Verificações: `pnpm --filter desktop test -- features components/SidePanel`; `pnpm --filter desktop typecheck`; `pnpm --filter desktop lint`. Se Rust mínimo for necessário, teste focalizado, não suíte GPU bloqueada.
6. AT-3.11: app Tauri real por `pnpm --filter desktop tauri dev`, resize UIA1280→1100→960 e painéis380→340→300px; capturas seguras e ausência de corte. Viewport Playwright é complementar, não substitui redimensionar janela Tauri. Falta atual de harness é trabalho de automação, não autorização para pedir operação humana.
7. Pronto: abas/controles/atalhos/responsividade e AT-3.11 cobertos, checks verdes; AT-3.01 antigo não cobre resize/tabs. Commit: `feat(desktop): painel lateral com 4 abas (F3-T14a2)`.

### F3-T14c — catálogo, axe e capturas (condicionada a T14a2/T12/T05)

**Estimativa:** ~2,5 h + E2E/triagem. **Arquivos:** `app/Catalog.tsx`, `src/main.tsx`, `e2e/catalog-desktop.spec.ts`, `playwright.config.ts` se faltar projeto1100px, `docs/ui/screens/desktop/catalog-{960,1100,1280}.png` e snapshots.

1. Inventariar componentes shared/desktop/shadcn usados: StatusChip, StatTile, SegmentedControl, SettingRow, DeviceRow, EmptyState, LevelMeter, HealthIndicator, PreviewFrame, QRCard e ApprovalDialog; não criar segunda biblioteca nem exportar internos sem necessidade.
2. Fixtures sintéticas com vazio/loading/normal/erro/disabled/texto longo pt-BR/pseudo+40%. Usar mock Tauri/sentinela QR, nunca app real/credencial. Algoritmo pseudo de teste serve de referência, não import `.test` em produção.
3. Rota somente DEV + `?catalog`, seguindo precedente mobile; import deve desaparecer no build. IDs estáveis e seções semânticas, sem iniciar sessão real.
4. UT-3.26: axe sem critical/serious. UT-3.27: capturas determinísticas960/1100/1280 com reduced motion. AT-3.12 automatizável: foco/ordem/ARIA, alvos desktop≥28px, contrasteAA, pseudo+40%,960×640 sem overflow, sem backdrop-filter no preview e≤2 camadas de blur.
5. Verificações: `pnpm exec playwright test catalog-desktop`; `pnpm --filter desktop build`; `Select-String -Path "desktop\dist\assets\*.js" -Pattern "catalog" -SimpleMatch` sem correspondência. Projeto desktop não roda cenário mobile por engano.
6. Catálogo em browser não aprova partes dependentes de Tauri/aparelho/leitor de tela real. Registrar pendências no destino de relatório de gate quando definido, sem inventá-lo nem pedir teste manual. Se novas chaves, apenas verificação i18n afetada.
7. Pronto: UTs/capturas/DCE e matriz automatizável cobertos, pendências reais explícitas, T05 concluída (Esc atual não pode ser contornado). Commit: `test(desktop): catalogo de componentes + capturas e2e (F3-T14c)`.

### F3-T17 — instrumentação e desempenho (condicionada a T10/T14a2)

**Estimativa:** ~3 h + medições/soaks, sem total fechado. **Arquivos:** `src-tauri/src/metrics.rs`, integrações mínimas decode/compose/preview/commands, `src/lib/preview-player.ts`, `features/settings/SettingsTab.tsx`, `src-tauri/tests/metrics.rs`, `scripts/soak-f3.ps1`, `docs/perf/fase-3.md` e CSVs de evidência.

1. Ring de quatro timestamps monotônicos/frame, p50/p95 por estágio/total, fps/drops; draw stats no player. Teste sintético de percentis conhecidos: `cargo test -p desktop --test metrics`.
2. Implementar relatórios/comandos previstos (`get_latency_report`, `get_preview_draw_stats`, `export_perf_report`), respeitando `LOCALCAMERA_DOCS_DIR` e fallback app-data com warn; habilitar Exportar diagnóstico sem segredos.
3. Sampler de15s, cultura invariável e saída incremental. Reutilizar padrão F1, não seus limites WS/handles. GPU por PDH filtrado `pid_<PID>` em processo separado; set indisponível não é valor zero nem autorização para reexecutar IT-3.04.
4. PT-3.01: CPU1080p30 por10min≤15% em PC6núcleos; PT-3.02 recepção→composto p95≤15ms; PT-3.03 recepção→canvas p95≤50ms. Ambiente/fixture/build precisam constar no relatório.
5. PT-3.04/AT-3.05: vidro-a-vidro por Wi-Fi alvo≤250ms com método/correlação documentados. Mock sem aparelho/rede equivalente não aprova critério real. PT-3.06 registra GPU sem picos100%; PT-3.07 memória estável10min.
6. PT-3.05 reutiliza T10≤4renders/s e PT-3.08 reutiliza cold start T01≤2s, somente após conferir artefato/versão; planejamento não confirma equivalência. AT-3.04 evidencia~30fps.
7. AT-3.10:30min sem crescimento de memória/quedafps; LT-3.01:uma execução60min (~70min reais), WS≤+20MB/h, handles/fps estáveis e sem frames vazados. Anunciar durações e confirmar ausência de evidência equivalente antes de iniciar; não repetir por intervalo/formato.
8. Verificação final `Test-Path "docs\perf\fase-3.md"` True não basta: tabelas/CSV/IDs e botão de exportação devem estar completos; metas perdidas são desvios para decisão/ADR no gate, não conformidade. Reconfirmar decisão B conforme cartão.
9. Commit: `feat(desktop): instrumentacao de latencia + docs/perf/fase-3.md (F3-T17)`.

### R-02 — README e raiz

**Entrada:** R-01 concluída; selecionar só depois das tarefas anteriores elegíveis. **Arquivos:** `README.md`, mover `soak-f1.csv` para `docs/perf/soak-f1.csv`, atualizar somente referências encontradas.

1. Descrição factual Android→webcam/mic Windows11 Wi-Fi/USB; pré-requisitos a partir de `.nvmrc`, `package.json`, `rust-toolchain.toml`, wrapper Gradle e build Android, sem versões presumidas. Distinguir alvo JVM17 de requisito de JDK dos plugins/configuração gerada.
2. Documentar apenas scripts/binários reais e mapa conciso das pastas. Confirmar `pnpm install --frozen-lockfile`, lint/typecheck/test/e2e/vectors/ci no manifesto antes de descrevê-los; Prettier não vira script por suposição.
3. Mover CSV preservando conteúdo; buscar `soak-f1.csv` inclusive referências ocultas relevantes e corrigir caminhos. Verificação prescrita: busca textual + executar comandos aplicáveis do README, reutilizando evidência aprovada equivalente antes de repetir suites.
4. Não disparar soak/perf nem suíte GPU historicamente bloqueada como efeito de verificar documentação; se um comando exigido depende do bloqueio, registrá-lo sem inventar verde.
5. **D4:** ZIP permanece intacto até autorização explícita. Sem ela, item de remoção permanece parcial/bloqueado; não considerar R-03 automaticamente liberada.
6. Pronto: README verificável, CSV/referências corretos, remoção somente autorizada. Commit: `docs: atualizar README e organizar evidência de soak (R-02)`.

### R-03 — CI Windows no GitHub Actions

**Entrada:** R-02 concluída; se parcial por D4, aguardar decisão de escopo/dependência. **Arquivo:** workflow novo em `.github/workflows/`, alinhado a `scripts/ci.ps1` e manifestos.

1. Instalação/cache determinísticos a partir de versões/lockfiles realmente fixados; conferir actions/APIs/documentação antes de escrever YAML, sem versões flutuantes ou instalação especulativa.
2. Checks exigidos: lint, typecheck, Vitest, Prettier `--check`, cargo fmt/clippy/test, vectors e Playwright e2e. `scripts/ci.ps1` não contém todos: acomodar explicitamente no workflow sem mudar escopo local incidentalmente.
3. Sem Android instrumentado, soak ou perf, sem segredos; permissões mínimas. Não instalar/incluir audit/deny por conveniência enquanto D3 pendente.
4. Verificar comandos locais uma vez quando necessário e checks no PR; evidência histórica de outra tarefa não aprova workflow novo. GPU/fixtures/ambiente hosted devem ser compatibilizados sem ocultar testes nem aceitar falha tácita.
5. Pronto exige PR verde e comandos locais preservados. Publicar/push/acionar CI externo depende de acesso e autorização aplicáveis; não inventar execução remota. Commit: `ci: adicionar GitHub Actions para Windows (R-03)`.

### R-04 — threat model LAN e auditorias

**Entrada:** sem dependência formal, mas respeitar seleção do progresso. **Arquivo:** `docs/security/threat-model-lan.md`.

1. Inspeção direcionada: `crates/lc-server/src/handshake/{perform,auth,approval}.rs`, `trust/{store,secrets,ratelimit}.rs`, config `lib.rs`, parser/protocolo; no Android, `service/HelloBuilder.kt`, `net/PrimaryConnection.kt` e `store/PairingStore.kt`; no desktop, commands/lib, capability e CSP. Confirmar funções/limites efetivos antes de redigir números.
2. Documentar o que terceiro na mesma LAN observa/tenta, onde credenciais trafegam/repousam, o que aprovação protege e o que não protege, com distinção entre fato, inferência e limite.
3. ST-3.01…05 são evidência histórica do smoke F3-T13 (PID desktop, entradas, capabilities, título/logs/release), não auditoria completa de tráfego ou WebView2. Não generalizar “sem conexão externa do PID desktop” para todos os processos.
4. Executar `cargo audit` e `cargo deny check licenses` apenas se disponíveis. Instalação depende D3; se não autorizada, registrar **bloqueado**, não aprovado. Registrar versões/comandos/resultados sem segredos.
5. Pronto: threat model revisado contra código e auditorias executadas; bloqueio de instalação não satisfaz aprovação dos checks. Commit: `docs(security): registrar threat model da LAN e auditorias (R-04)`.

**Pontos factuais para confirmar no documento:** HELLO recebe token/chave e o caminho Android consultado usa `java.net.Socket`, sem TLS observado; isso sugere exposição a captura LAN, não prova captura realizada nem cobre toda rota. Desktop usa RNG/SHA-256 e comparação constante; persiste token de pareamento e hash da chave em `trust.json`, sem comprovar ACL do arquivo. Android separa metadados JSON da chave via SecureStore/Keystore, mas backup/garantias criptográficas exigem leitura específica. Aprovação exige token válido, limita pendência e distingue once/always/reject; ID revogado resulta em `Revoked`. Registrar também a limitação observada de sessão continuar se a persistência da rotação falhar. Capability/CSP e registro de comandos não provam ACL individual por comando nem isolamento contra frontend comprometido. Limites de payload e números efetivos de rate-limit ainda precisam de confirmação direcionada.

## Coordenação, evidência e saída de cada tarefa

- Planejamento paralelo é somente leitura. Na implementação, declarar dono por arquivo; `Shell.tsx`, `commands.rs`, `events_pump.rs`, store/ponte e catálogos i18n são zonas compartilhadas e precisam ser serializados.
- Toda entrega registra código/versão efetivamente testados, comando, resultado, evidência segura, limitações e cleanup; o gerente revisa antes de `[x]` e commit. Dependência parcial não equivale a `[x]`.
- Não somar estimativas de cartões nem prometer prazo total: há decisões externas, verificação de APIs, testes longos e risco de hardware. Custos reais devem ser atualizados por bloco concluído.
- Ao concluir o cartão ativo, atualizar **Próxima tarefa** pela primeira elegível na tabela real, não por uma contagem fixa de tarefas ou por disponibilidade de workers.
