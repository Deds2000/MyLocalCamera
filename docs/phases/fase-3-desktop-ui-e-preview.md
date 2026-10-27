# Fase 3 — Desktop: UI, decodificação e preview

**Marco:** M1 · **Tamanho:** G (≈ 12–18 dias) · **Depende de:** Fase 0 (S5), Fase 1; Fase 2 para testar com celular real
**Relatório:** `docs/tests/fase-3.md`

## 1. Objetivo e resultado observável
Janela única completa (dark glassmorphism), com estado vazio e QR, diálogo de aprovação, dispositivos confiáveis, **decodificação H.264**, **composição** (rotação/espelho/escala) e **preview fluido** pela estratégia vencedora do S5, respeitando as regras de desempenho da UI.

**Resultado observável:** com o celular (ou o mock) conectado, a janela mostra QR → aprovação → preview fluido em 1080p30 com chips corretos; o React Profiler não registra renderizações por quadro.

## 2. Leitura obrigatória
`docs/ui-spec.md` (design system, §4 desempenho, §8 checklist), **`docs/ui/desktop.md`** (inteiro), `docs/ui/copy.md`, `docs/spikes/S5.md`, `docs/protocol.md` §6 e §10, `docs/testing.md`.

## 3. Escopo
**Dentro:** shell da janela, painel lateral com as quatro abas (controles ainda inativos), estado vazio/QR, aprovação, confiáveis, decodificação, composição de um stream, preview, estatísticas a 2 Hz, `HealthBar` (servidor funcional), catálogo de componentes, `docs/composition.md`.
**Fora:** câmera virtual (4), USB/mDNS (5), controles remotos (6), PiP (7), áudio (8).

## 4. Entradas e saídas
| Entradas | Saídas |
|---|---|
| Servidor da Fase 1, mock-sender, fixtures, veredito do S5 | Desktop com UI; módulos `decode/`, `compose/`, `preview/`; `docs/composition.md`; textos i18n; capturas em `docs/ui/screens/desktop/`; `docs/perf/fase-3.md`; `docs/tests/fase-3.md` |

## 5. Plano de trabalho

### Etapa A — Shell e design system
- [ ] **F3-T01 — Janela e titlebar** (`ui/desktop.md` §2.1, §2.4): sem decoração; região de arrasto (duplo clique maximiza); pílula de estado; menu de idioma; tamanho inicial 1280×800, mínimo 960×640; **breakpoints** do painel (380/340/300 px); instância única; **lembrar tamanho e posição** (e validar se a posição salva ainda está em um monitor existente); fundo com gradiente e glows (`ui-spec` §1.3).
- [ ] **F3-T02 — Componentes e animações:** shadcn/ui necessários + variantes de glass; framer-motion (troca de seções, cartões, pulso) com `prefers-reduced-motion`; componentes próprios (`StatusChip`, `StatTile`, `SegmentedControl`, `SettingRow`, `DeviceRow`, `EmptyState`, `LevelMeter`, `HealthIndicator`, `PreviewFrame`, `QRCard`, `ApprovalDialog`).
- [ ] **F3-T03 / T14b — i18n:** copiar `common.*` e `desktop.*` de `ui/copy.md` para os JSON; lint `no-literal-string` verde.

### Etapa B — Estado e ponte com o Rust
- [ ] **F3-T13 — Comandos e eventos Tauri** (tipados em `lib/tauri.ts`): `get_connection_info`, `regenerate_token`, `approve/reject`, `list_trusted`, `revoke_trusted`, `get_health`; eventos `session://connection`, `session://stats`, `trust://pending`. **Validar entradas no Rust**; capacidades do Tauri com privilégio mínimo.
- [ ] **F3-T14a — Store (zustand):** slices `connection`, `cameraState`, `stats`, `trust`, `usb`, `health`, `settings`, `ui` (ver `ui/desktop.md` §2.11), com seletores estreitos.

### Etapa C — Conexão, aprovação e confiança (UI)
- [ ] **F3-T04 — Estado vazio e aba Conexão** (`ui/desktop.md` §2.3 e §2.6.1): `QRCard` (`qrcode.react`, margem clara) gerado da URI de pareamento; seletor de interface (priorizar adaptadores reais; esconder virtuais óbvios por padrão); **Regenerar QR** com confirmação; código mascarado (mostrar/copiar); seção USB (estrutura pronta; dados na Fase 5).
- [ ] **F3-T05 — `ApprovalDialog`** (§2.8.1): não fecha com `Esc`/clique fora (equivale a Recusar); foco inicial em **Recusar**; contagem de 30 s; **traz a janela ao primeiro plano e notifica o Windows** se estiver minimizada; segundo pedido simultâneo recusado (`busy`).
- [ ] **F3-T06 — Dispositivos confiáveis:** listar, "visto por último", **Remover** com confirmação.

### Etapa D — Decodificação e composição
- [ ] **F3-T07 — `decode/`:** decodificador H.264 de **Media Foundation** (MFT) com aceleração por GPU e **fallback por software** (forçável por variável de ambiente para teste); baixa latência; saída NV12; **esperar o primeiro keyframe**; reinício ao receber SPS/PPS novo; após erro, pular até o próximo keyframe e emitir `REQUEST_KEYFRAME`; fila limitada (descartar quadros antigos se o consumidor atrasar).
- [ ] **F3-T08 — `compose/` e `docs/composition.md`:** especificar e implementar a ordem das transformações (**rotação (hint + usuário) → espelho → escala `contain`/`cover` → letterbox preto → saída NV12 1920×1080**), arredondamento de coordenadas, tratamento de proporções (16:9, 4:3, retrato), e **definição única** usada por preview e câmera virtual. Implementação inicial em CPU (SIMD se necessário); interface preparada para GPU/PiP (Fase 7).

### Etapa E — Preview (conforme S5)
- [ ] **F3-T09 — Preview**
  - *Se A (WebCodecs):* `Channel` binário com os *access units*; `preview-player.ts` com `VideoDecoder` (`avc: { format:'annexb' }`); reinício no próximo keyframe após erro; transformações no canvas **segundo `docs/composition.md`**.
  - *Se B (preview reduzido):* Rust gera JPEG ~960×540 a 30 fps **do mesmo quadro composto**; frontend usa `createImageBitmap` e desenha no canvas.
  - Comum: fila limitada com **descarte do mais antigo**; pausa quando a janela está oculta; sem alocações por quadro no JS (reutilizar buffers).
- [ ] **F3-T10 — Regras de desempenho** (`ui-spec` §4): `session://stats` a 2 Hz (agregado no Rust); desenho **imperativo** do canvas; seletores estreitos; pausar ao minimizar (a decodificação para a câmera virtual continua).
- [ ] **F3-T11 — Estados, chips e barra de ferramentas** (`ui/desktop.md` §2.5): estados do preview, chips (latência `warn` > 80 ms e `danger` > 200 ms), atalhos (§2.9), espelho/giro com efeito local imediato (o `SET` vem na Fase 6), tela cheia.

### Etapa F — Painel, saúde e qualidade
- [ ] **F3-T14a2 — Painel lateral** (§2.6): quatro abas com `SettingRow`; controles ainda não funcionais ficam **desabilitados com tooltip de motivo**; árvore de componentes conforme §2.11.
- [ ] **F3-T12 — `HealthBar`** (§2.7): **Servidor** funcional; os demais "Em breve"; popovers de correção.
- [ ] **F3-T14c — Catálogo `?catalog`** com todos os componentes e estados; capturas em `docs/ui/screens/desktop/`.
- [ ] **F3-T17 — Instrumentação de latência:** log/métrica interna do tempo **recepção → decodificado → composto → preview**; botão de diagnóstico que exporta as últimas medições para `docs/perf/fase-3.md`.

## 6. Armadilhas conhecidas
- Decodificadores de hardware variam entre GPUs: sempre ter o *fallback* por software e um teste que o force.
- `ImageBitmap`/`VideoFrame` precisam ser **fechados** (`close()`), senão vazam memória de GPU.
- DPI em 125%/150% pode borrar o canvas se `devicePixelRatio` não for tratado.
- Janela sem decoração perde *snap layouts* se mal configurada (verificar).
- O WebView2 pode limitar timers em janela oculta: não depender deles para lógica de negócio.
- Mudança de resolução no meio do stream (novo SPS) exige reiniciar decodificador **sem** piscar a imagem (mostrar o último quadro até o próximo keyframe).

## 7. Plano de testes

### 7.1 Unitários
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-3.01 | `compose` (golden): rotação 0/90/180/270 × espelho × `contain`/`cover` × entrada 16:9, 4:3 e retrato, com padrão sintético (4 quadrantes + marcadores) | Saída idêntica à referência gerada de `docs/composition.md` (checagem por *hash* e por pixels-chave) | ✅ |
| UT-3.02 | `compose`: `rotationHint` + rotação do usuário somados módulo 360 | Resultado correto para todas as combinações | ✅ |
| UT-3.03 | `compose`: espelho aplicado **após** a rotação | Eixo correto (horizontal na imagem final) | ✅ |
| UT-3.04 | `compose`: letterbox | Barras pretas simétricas; sem 1 px de erro | ✅ |
| UT-3.05 | Decoder (software, com fixtures): `hd30.h264` | N quadros decodificados, resolução correta | 🔧 |
| UT-3.06 | Decoder: `no-idr-start.h264` | Descarta até o primeiro keyframe; sem *crash* | 🔧 |
| UT-3.07 | Decoder: `corrupt.h264` | Recupera no próximo keyframe; emite `REQUEST_KEYFRAME` | 🔧 |
| UT-3.08 | Decoder: `sps-change.h264` | Reinicia e continua; resolução nova refletida | 🔧 |
| UT-3.09 | Fila do preview (limitada): consumidor lento | Descarta os mais antigos; memória constante | ✅ |
| UT-3.10 | Agregador de estatísticas: a cada 500 ms, valores corretos (fps, bitrate, RTT, perdidos) | Conforme entradas simuladas | ✅ |
| UT-3.11 | Store: seletores estreitos (um slice muda, outros não re-renderizam) | Contagem de renders esperada | ✅ |
| UT-3.12 | Config. de interface de rede: filtro de adaptadores virtuais | Reais primeiro; escolha do usuário respeitada | ✅ |
| UT-3.13 | Geração da URI do QR (hosts, porta, token, pc, name; codificação de caracteres especiais no nome) | URI válida e *parseável* pelo parser do mobile (vetor compartilhado) | ✅ |
| UT-3.14 | Posição/tamanho da janela salvos e restaurados; monitor ausente | Volta para posição válida | ✅ |

### 7.2 Testes de UI (RTL, axe, Playwright)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-3.20 | Estado vazio, conectando, pendente, transmitindo, reconectando, erro | Cada estado do preview conforme §2.5.2 | ✅ |
| UT-3.21 | `ApprovalDialog`: `Esc`, clique fora, `Enter` imediato, Tab | Nenhum aprova; foco em Recusar; contagem regressiva fecha como não aprovado | ✅ |
| UT-3.22 | Lista de confiáveis: remover com confirmação | Item sai; `revoke_trusted` chamado | ✅ |
| UT-3.23 | Aba Conexão: regenerar QR (confirma/cancela) | QR muda só ao confirmar | ✅ |
| UT-3.24 | Atalhos de tecla única com foco em campo de texto | Ignorados | ✅ |
| UT-3.25 | Cobertura de i18n e textos longos (pseudo-localização +40%) | Nenhum corte/estouro | ✅ |
| UT-3.26 | axe nas telas do catálogo | Sem violações sérias | 🔧 |
| UT-3.27 | Capturas do `?catalog` (Playwright) em 960, 1100 e 1280 px | Sem diferenças não aprovadas | 🔧 |

### 7.3 Integração
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-3.01 | mock-sender (`pattern1080.h264`) → servidor → decoder → compose → preview (headless) | Quadros chegam na ordem; contador do `lc-pattern` sem saltos | 🔧 |
| IT-3.02 | Mock com `--rotation-hint 90` e `--size` 4:3 | Saída conforme `composition.md` | 🔧 |
| IT-3.03 | Fluxo completo de pareamento: mock → `trust://pending` → UI aprova → vídeo | Evento e diálogo; vídeo após aprovar | 🔧 |
| IT-3.04 | Decoder em hardware **e** forçando software | Mesmo resultado visual (tolerância definida) | 🔧 |
| IT-3.05 | (Opcional) `tauri-driver`: aprovar dispositivo e regenerar QR | Fluxo passa | 🔧 |

### 7.4 Aceitação
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-3.01 | Abrir o desktop sem celular | Estado vazio com QR, IPs e botão regenerar; QR muda ao regenerar | ✋ |
| AT-3.02 | Celular novo escaneia o QR | Diálogo em < 1 s; "Permitir sempre" salva o dispositivo | ✋ |
| AT-3.03 | Revogar o dispositivo na lista | Próxima conexão recebe `revoked` | ✋ |
| AT-3.04 | Transmitir do celular real em 1080p30 | Preview fluido ≈ 30 fps; chips corretos | ✋ |
| AT-3.05 | Latência com cronômetro | Wi-Fi ≤ 250 ms (meta); valor em `docs/perf/fase-3.md` | ✋ |
| AT-3.06 | Minimizar por 1 min e restaurar | CPU cai ao minimizar; preview retoma sem travar | ✋ |
| AT-3.07 | Espelhar e girar 90/180/270 | Igual a `docs/composition.md` | ✋ |
| AT-3.08 | React Profiler durante o stream | Nenhuma renderização por quadro; estatísticas a ≤ 2 Hz | ✋ |
| AT-3.09 | Trocar idioma pt-BR ↔ en em tempo real | Todos os textos mudam | ✋ |
| AT-3.10 | Rodar 30 min | Sem crescimento de memória; sem queda de fps | ✋ |
| AT-3.11 | Redimensionar de 1280 a 960 px | Painel 380→340→300 px; abas só com ícone; nada corta | ✋ |
| AT-3.12 | Checklist de `ui-spec` §8 (teclado, leitor de tela, pt-BR e en) | Todos os itens marcados | ✋ |
| AT-3.13 | Diálogo de aprovação: `Esc`, clique fora, `Enter` | Nenhum aprova; foco em Recusar | ✋ |
| AT-3.14 | Atalhos de `ui/desktop.md` §2.9 só com teclado | Ações do preview acessíveis | ✋ |
| AT-3.15 | DPI do Windows em 100%, 150% e 200% | Nitidez e layout corretos; QR legível por outro celular | ✋ |
| AT-3.16 | Mover a janela entre dois monitores de DPIs diferentes | Sem artefatos; canvas reajusta | ✋ |
| AT-3.17 | Segundo plano: abrir o app com a janela minimizada e chegar um pedido de aprovação | Janela vem à frente + notificação do Windows | ✋ |

### 7.5 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-3.01 | `--corrupt 5` (5% dos quadros) por 2 min | Recuperações automáticas; toast "Imagem retomada" sem spam; sem *crash* | 🔧 |
| FT-3.02 | `--loss 10` | Idem; keyframes pedidos | 🔧 |
| FT-3.03 | Troca de resolução 720p→1080p no meio (`sps-change`) | Sem piscar para preto (mantém último quadro até o keyframe) | 🔧 |
| FT-3.04 | Celular some (`--kill-after`) | Preview "Reconectando" com último quadro; depois erro; sem travar a UI | 🔧 |
| FT-3.05 | Decodificador em hardware indisponível | *Fallback* de software automático; aviso discreto; fps ainda aceitável | 🔧 |
| FT-3.06 | Consumidor do preview lento (CPU saturada) | Descarta quadros; UI responsiva | 🔧 |
| FT-3.07 | Dois pedidos de aprovação ao mesmo tempo | Segundo recebe `busy`; sem diálogos empilhados | 🔧 |
| FT-3.08 | Porta ocupada ao iniciar | Banner/rodapé em `warn` com porta efetiva; QR usa a porta efetiva | 🔧 |
| FT-3.09 | Arquivo de configurações corrompido | App abre com padrões; aviso; sem *crash* | ✅ |
| FT-3.10 | Fechar a janela durante um pedido de aprovação | Pedido recusado; celular recebe `not_approved` | 🔧 |

### 7.6 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-3.01 | CPU total do desktop com 1080p30 (decodificação por GPU + composição + preview) | ≤ 15% de um PC de 6 núcleos (registrar) | ✋ |
| PT-3.02 | Latência interna recepção → composto, p95 | ≤ 15 ms | 🔧 |
| PT-3.03 | Latência do preview (recepção → canvas), p95 | ≤ 50 ms | 🔧 |
| PT-3.04 | Latência vidro a vidro (cronômetro) | ≤ 250 ms Wi-Fi | ✋ |
| PT-3.05 | Renderizações do React durante o stream | ≤ 4/s | ✋ |
| PT-3.06 | Estimativa de GPU (Gerenciador de Tarefas) | Registrar; sem picos de 100% | ✋ |
| PT-3.07 | Memória do processo (UI + Rust) após 10 min | Estável | ✋ |
| PT-3.08 | Partida a frio do desktop | ≤ 2 s até a janela utilizável | ✋ |

### 7.7 Segurança
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-3.01 | Tráfego do WebView durante o uso | **Nenhuma** requisição externa; CSP aplicada | ✋ |
| ST-3.02 | Capacidades do Tauri | Só comandos/eventos usados estão permitidos | ✅ |
| ST-3.03 | Comandos com entradas inválidas (porta, IDs, strings gigantes) | Rejeitados no Rust sem *panic* | ✅ |
| ST-3.04 | Token/chave em logs, DevTools e títulos de janela | Ausentes | ✋ |
| ST-3.05 | DevTools em release | Desativadas | ✋ |
| ST-3.06 | Diálogo de aprovação: tentativa de aprovar via IPC sem pedido pendente | Recusado | ✅ |

### 7.8 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-3.01 | **60 min** de stream (mock ou celular) com `soak.ps1` | Memória ≤ +20 MB/h; *handles* estáveis; fps estável; sem vazamento de `VideoFrame/ImageBitmap` | 🔧 |
| LT-3.02 | 100 ciclos minimizar/restaurar | Sem vazamento; preview sempre retoma | 🔧 |

### 7.9 Regressão
**RS-S** completa + AT-1.xx e AT-2.xx (smoke).

## 8. Checklist de saída
- [ ] Testes obrigatórios aprovados; metas de PT registradas (ou desvio por ADR).
- [ ] `docs/composition.md` completo; capturas do catálogo atualizadas.
- [ ] `docs/perf/fase-3.md` com latência, CPU/GPU e decisão final do preview.
- [ ] `docs/tests/fase-3.md` preenchido.

## 9. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| MFT H.264 instável em alguma GPU | Alternativa: FFmpeg ligado dinamicamente (LGPL); ADR |
| Preview A consumindo CPU demais | Trocar para B (já medido no S5) |
| Janela sem decoração com comportamento estranho no Windows 11 | Usar decoração nativa + titlebar escondida; ADR |
