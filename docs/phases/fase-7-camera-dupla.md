# Fase 7 — Câmera dupla (marco M3)

**Marco:** M3 → tag `v0.3.0` · **Tamanho:** G (≈ 12–18 dias) · **Depende de:** Fase 6, `docs/spikes/S3.md`
**Relatório:** `docs/tests/fase-7.md`

## 1. Objetivo e resultado observável
Transmitir **frontal e traseira ao mesmo tempo** (em aparelhos que suportem). No PC, o usuário escolhe nas configurações entre **duas webcams separadas** ("LocalCamera Back" e "LocalCamera Front") ou **uma webcam com picture-in-picture**.

**Resultado observável:** nos dois modos, as câmeras virtuais corretas funcionam em apps reais; em aparelho sem suporte, a opção "Dupla" não aparece.

## 2. Leitura obrigatória
`docs/spikes/S3.md` (limites reais do aparelho), `docs/protocol.md` §9–§10, `docs/composition.md`, `docs/ui/desktop.md` §2.5.5 e §2.6.2, `docs/ui/mobile.md` §3.8, `docs/phases/fase-4-camera-virtual.md` (projeto da DLL), `docs/testing.md`.

## 3. Escopo
**Dentro:** captura simultânea; dois encoders e streams (1 e 2); rebaixamento automático; duas câmeras virtuais **ou** PiP; troca principal/miniatura; UI nos dois apps; `mock-sender --dual`; atualização de `docs/devices.md`.
**Fora:** seleção de lente (backlog).

## 4. Entradas e saídas
| Entradas | Saídas |
|---|---|
| S3 (modo simultâneo), Fase 6, DLL e `vcam-probe` da Fase 4 | Modo dual no plugin e no desktop; duas classes COM na DLL; compositor de PiP; `docs/devices.md` com a matriz de aparelhos; `docs/tests/fase-7.md`; tag `v0.3.0` |

## 5. Plano de trabalho

### Etapa A — Celular
- [ ] **F7-T01 — Captura simultânea** em `CameraBackend` conforme o S3 (CameraX com a API de câmeras simultâneas **ou** Camera2 com `getConcurrentCameraIds()` e duas sessões). Em serviço, usar `LifecycleService` quando for CameraX.
- [ ] **F7-T02 — Dois `VideoEncoder`** independentes (stream 1 = traseira, 2 = frontal), cada um com `STREAM_CONFIG`, keyframes e bitrate próprios (padrão ~4 Mbps em 720p30; respeitar `caps.concurrentModes`). **Limite de codecs simultâneos do aparelho:** se o segundo encoder falhar ao iniciar, tentar resolução menor; se ainda falhar, voltar a `single` com `rejected: failed`.
- [ ] **F7-T03 — Rebaixamento automático:** se a combinação pedida não for aceita, reduzir ao **maior modo simultâneo permitido** e informar o valor real em `STREAM_CONFIG` e `STATE`.
- [ ] **F7-T04 — `caps.concurrent`** verdadeiro **somente** se o teste do S3 passou naquele aparelho; a UI esconde "Dupla" no celular e no PC caso contrário.
- [ ] **F7-T05 — `SendQueue` por stream:** política de descarte **independente**; um stream lento não prejudica o outro nem o controle; reservar prioridade para o controle.
- [ ] **F7-T06 — Interrupções:** chamada, outro app abrindo a câmera, superaquecimento → voltar a `single` com aviso (`rejected: busy`/`failed`) **sem derrubar a conexão**.
- [ ] **F7-T07 — Térmica e energia:** monitorar o estado térmico; ao atingir limite, reduzir bitrate/resolução ou voltar a `single`, com aviso (`mobile.stream.warnHot`).

### Etapa B — Desktop
- [ ] **F7-T08 — Dois decodificadores** em paralelo, com recuperação independente por stream; estado "stream sem quadros há > 1 s".
- [ ] **F7-T09 — Compositor PiP** (`compose/` + `docs/composition.md`): principal em tela cheia + miniatura (tamanho 15–35%, padrão 25%; margem de 16 px a 1080p; raio de canto 24 px; borda de 2 px a 20% de branco), 4 cantos, **trocar principal/miniatura**; espelho/rotação **por câmera**; se a miniatura ficar sem quadros por > 1 s, **ocultá-la** (e reaparecer ao voltar); se o principal ficar sem quadros, mostrar o último quadro por até 1 s e depois o placeholder.
- [ ] **F7-T10 — Configuração "Saída no modo dual"** (Duas webcams × PiP; padrão PiP); persistida; aviso `desktop.camera.dualModeWarning` ao alternar.
- [ ] **F7-T11 — `vcam/` e `virtualcam/`: duas câmeras.** Duas classes COM (CLSIDs) na mesma DLL, cada uma com seu canal; "LocalCamera Back" e "LocalCamera Front". **Sequência segura de troca de modo:** parar de enviar → desligar/remover as câmeras do modo antigo → aguardar a confirmação → criar as do novo modo → só então retomar o envio. Apps abertos perdem o dispositivo (documentado).
- [ ] **F7-T12 — Preview:** PiP composto, **ou** duas molduras lado a lado (50/50) com legenda e seleção da moldura ativa (`ui/desktop.md` §2.5.5); miniatura PiP **arrastável** entre os 4 cantos.
- [ ] **F7-T13 — Rodapé:** estado de cada câmera virtual (registrada/em uso) no modo duas webcams.

### Etapa C — Ferramentas e qualidade
- [ ] **F7-T14 — `mock-sender --dual`:** dois streams com padrões diferentes (ex.: `pattern1080` para a traseira e `pattern720` com cores trocadas para a frontal), controles independentes de falha por stream (`--stall-b`, `--kill-b`).
- [ ] **F7-T15 — `vcam-probe`** ganha `--name "LocalCamera Back"/"Front"` e `--expect-pip` (valida a posição/tamanho da miniatura por amostragem de pixels).
- [ ] **F7-T16 — `docs/devices.md`:** matriz dos aparelhos testados (modelo, Android, resolução dual máxima, encoders, observações).
- [ ] **F7-T17 — Testes e relatório** (§7).

## 6. Armadilhas conhecidas
- Muitos aparelhos **não** permitem as duas câmeras ao mesmo tempo, ou só em resoluções baixas.
- Dois encoders de hardware podem não coexistir; a segunda instância pode cair para software e esquentar o aparelho.
- A frontal geralmente tem orientação/espelho diferente da traseira: respeitar `rotationHint` **por stream**.
- Recriar câmeras virtuais derruba os apps que as usavam; a UI avisa antes.
- No modo PiP, uma única câmera virtual: os apps não percebem a troca principal/miniatura (é só composição).

## 7. Plano de testes

### 7.1 Unitários
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-7.01 | Layout do PiP: 4 cantos × tamanhos 15/25/35% × saída 1080p | Retângulo, margem e raio corretos (golden) | ✅ |
| UT-7.02 | Máscara de cantos arredondados e borda | Pixels internos/externos conforme especificação | ✅ |
| UT-7.03 | Trocar principal/miniatura | Conteúdos trocados; espelho/rotação **acompanham a câmera** | ✅ |
| UT-7.04 | Mapa de streams (single: ativa=1; dual: traseira=1, frontal=2) | Correto em todas as transições | ✅ |
| UT-7.05 | Reducer: `single`↔`dual`, rejeição quando `concurrent=false`, rebaixamento | `STATE`/`rejected` corretos | ✅ |
| UT-7.06 | Escolha de modo simultâneo (dada `caps` + pedido) — tabela de casos | Maior modo permitido; nunca acima do limite | ✅ |
| UT-7.07 | `SendQueue` por stream: um stream lento não descarta o outro | Políticas independentes | ✅ |
| UT-7.08 | Gerenciador de câmeras virtuais: sequência de criação/remoção, nomes e CLSIDs por modo | Ordem segura; nunca duas com o mesmo nome | ✅ |
| UT-7.09 | Regras de "stream sem quadros" (miniatura some após 1 s; principal congela e vira placeholder) | Temporização correta (relógio virtual) | ✅ |
| UT-7.10 | RTL: opção Dupla desabilitada com motivo; escolha de saída; posição/tamanho da miniatura; aviso ao alternar | Conforme `ui/desktop.md` | ✅ |
| UT-7.11 | Posição da miniatura arrastada encaixa no canto mais próximo | Canto correto | ✅ |

### 7.2 Integração
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-7.01 | `mock --dual` + modo **PiP** → `vcam-probe --expect-pip` | Miniatura no canto/tamanho configurados; principal correto; **0 perdas** | 🔧 |
| IT-7.02 | `mock --dual` + **duas webcams** → dois `vcam-probe` | "Back" mostra o padrão A; "Front" mostra o B; sem troca | 🔧 |
| IT-7.03 | Alternar PiP ↔ duas webcams 20× com o mock rodando | Sempre termina no modo escolhido; sem dispositivos órfãos | 🔧 |
| IT-7.04 | `mock --dual --kill-b` | Miniatura some; principal intacto; câmera virtual contínua | 🔧 |
| IT-7.05 | `androidTest`: dois encoders 720p30 por 60 s (no aparelho) | Ambos produzem Annex-B válido com keyframes ≈ 1 s | 🔧 |
| IT-7.06 | `androidTest`: tentar `dual` em aparelho sem suporte (ou com `caps` simulados) | `rejected` e permanece em `single` | 🔧 |

### 7.3 Aceitação
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-7.01 | Aparelho **com** suporte: ativar Dupla | Dois streams chegam; resoluções reais exibidas | ✋ |
| AT-7.02 | Aparelho **sem** suporte (ou `caps` simulados) | "Dupla" não aparece no celular nem no PC | ✋ |
| AT-7.03 | Modo **PiP** em Zoom/Meet | Uma câmera "LocalCamera" com principal + miniatura corretas | ✋ |
| AT-7.04 | Trocar principal/miniatura e mover a miniatura | Efeito imediato, sem reiniciar captura | ✋ |
| AT-7.05 | Modo **duas webcams** | "LocalCamera Back" e "LocalCamera Front" funcionam em apps diferentes ao mesmo tempo | ✋ |
| AT-7.06 | Alternar entre os dois modos de saída | Câmeras recriadas sem travar o desktop; apps se recuperam ao reabrir | ✋ |
| AT-7.07 | Pedir resolução acima do limite simultâneo | Rebaixa automaticamente e informa | ✋ |
| AT-7.08 | Receber ligação ou abrir outro app de câmera no modo dual | Volta a `single` com aviso; conexão mantida | ✋ |
| AT-7.09 | 30 min em dual | Sem queda de fps; temperatura aceitável (registrar) | ✋ |
| AT-7.10 | NVIDIA Broadcast com a câmera PiP | Funciona como na Fase 4 | ✋ |
| AT-7.11 | Espelhar só a frontal e girar só a traseira (duas webcams e PiP) | Cada câmera respeita o seu ajuste | ✋ |
| AT-7.12 | OBS capturando "Back" e Zoom usando "Front" ao mesmo tempo | Ambos fluidos | ✋ |
| AT-7.13 | Preview em "duas webcams": selecionar moldura e usar os atalhos | Atalhos agem na moldura selecionada | ✋ |

### 7.4 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-7.01 | Um dos streams morre; depois o outro | Comportamento por stream conforme F7-T09; reconexão restaura | 🔧 |
| FT-7.02 | Segundo encoder falha ao iniciar | Tenta resolução menor; senão volta a `single` com aviso | 🔧 |
| FT-7.03 | Câmera frontal "desconectada" (outro app a toma) | Dual cai para single; mensagem | ✋ |
| FT-7.04 | Superaquecimento durante dual | Reduz qualidade/volta a single com aviso; sem desligar o app | ✋ |
| FT-7.05 | Alternar modo de saída 20× rapidamente | Sem câmeras fantasmas; último clique vence | 🔧 |
| FT-7.06 | Suspender/retomar o Windows em dual | Câmeras voltam em ≤ 5 s | ✋ |
| FT-7.07 | Desktop encerrado à força no modo duas webcams | Apps consumidores não travam | ✋ |
| FT-7.08 | Rede lenta (`--slow`) só em um dos streams | O outro não é afetado | 🔧 |

### 7.5 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-7.01 | CPU/GPU do desktop em 2×720p30 (PiP e separadas) | Registrar; ≤ 20% de CPU (PC de 6 núcleos) | ✋ |
| PT-7.02 | Tempo de composição do PiP por quadro | ≤ 4 ms (p95) | 🔧 |
| PT-7.03 | Variação do intervalo entre quadros (cada câmera virtual) | Desvio-padrão ≤ 3 ms | 🔧 |
| PT-7.04 | Latência de cada stream | ≤ 250 ms Wi-Fi | ✋ |
| PT-7.05 | Temperatura e bateria do celular em 30 min dual | Registrar; sem *throttling* que derrube o fps abaixo de 25 | ✋ |
| PT-7.06 | Tempo para alternar o modo de saída | ≤ 3 s | ✋ |

### 7.6 Segurança
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-7.01 | Os dois canais (pipes) têm a mesma DACL restrita da Fase 4 | Verificado | ✋ |
| ST-7.02 | Mensagens de `stream` inexistente (ex.: 5) ou duplicado | Ignoradas/recusadas sem *crash* | ✅ |

### 7.7 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-7.01 | **60 min** em dual (mock) com `vcam-probe` em ambos os modos | Sem perdas; memória/handles estáveis | 🔧 |
| LT-7.02 | 100 alternâncias de modo de saída | Sem câmeras órfãs; sem vazamento | 🔧 |
| LT-7.03 | 30 min no celular real em dual | Estável; temperatura registrada | ✋ |

### 7.8 Regressão
**RS-S** + AT-4 (matriz de apps no modo PiP) + AT-6 (controles) e o **RS-F** antes de `v0.3.0`.

## 8. Entrega do marco M3 (`v0.3.0`)
- [ ] Testes obrigatórios aprovados; `docs/devices.md` atualizado; `docs/tests/fase-7.md` aprovado; tag `v0.3.0`.

## 9. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| Aparelho do usuário não suporta dual | Validar com `caps` simulados e, se possível, emprestar um aparelho compatível; recurso permanece condicional |
| CPU alta com dois decodificadores | Compor/escala em GPU (D3D11); reduzir fps do preview |
| Apps não toleram a recriação das câmeras | Manter modo fixo por sessão e exigir reinício do app consumidor (documentar) |
