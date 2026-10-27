# Fase 6 — Controle bidirecional e apresentação (fecha o marco M2)

**Marco:** M2 → tag `v0.2.0` · **Tamanho:** M (≈ 8–12 dias) · **Depende de:** Fases 2–5
**Relatório:** `docs/tests/fase-6.md`

## 1. Objetivo e resultado observável
Os **dois lados controlam** a câmera e o **último comando vence**; o resultado converge nas duas telas. Espelho e rotação são aplicados no PC, instantaneamente. O bitrate se adapta à rede.

**Resultado observável:** trocar câmera/resolução/fps/bitrate no PC **e** no celular funciona; alterar a mesma opção nos dois lados quase ao mesmo tempo termina com o **mesmo valor** em ambos; a câmera virtual não pisca durante mudanças.

## 2. Leitura obrigatória
`docs/protocol.md` §9–§11, `docs/composition.md`, `docs/ui/desktop.md` §2.5.4 e §2.6.2, `docs/ui/mobile.md` §3.8, `docs/testing.md` (§4.1: `--emulate-phone`, `--chaos`).

## 3. Escopo
**Dentro:** `SET`/`STATE` completos; reducer único no celular; atualização otimista + reconciliação no PC; troca de câmera, resolução, fps e bitrate; espelho e rotação por câmera; `STATS` e bitrate adaptativo; `REQUEST_KEYFRAME`; persistência de preferências; `mock-sender --emulate-phone`.
**Fora:** modo dual (7), áudio (8).

## 4. Mudança de protocolo desta fase (minor, compatível → **1.1**)
Acrescentar ao `STATE` o campo opcional **`rejected: [{ key, reason }]`** (`reason` ∈ `unsupported`, `out_of_range`, `busy`, `failed`) para o celular informar por que um `SET` não foi aplicado. Atualizar `docs/protocol.md`, vetores e os três lados **no mesmo commit**. Clientes antigos ignoram o campo.

## 5. Plano de trabalho

### Etapa A — Reducer e validação (celular)
- [ ] **F6-T01 — `StateReducer.kt`:** aplica `SET`s **e** mudanças locais pelo mesmo caminho, na ordem de chegada; valida contra `caps` (modos, fps, faixas de bitrate); **no-op não incrementa `rev`**; mudança aceita incrementa `rev` e publica `STATE`; rejeição publica o `STATE` corrigido com `rejected`; `SET` com lista aplica de forma **atômica** (tudo ou nada).
- [ ] **F6-T02 — Reconfiguração de captura sem derrubar a conexão:** parar encoder/câmera, reconfigurar, novo `STREAM_CONFIG` + keyframe; se falhar, **reverter** para a configuração anterior e publicar `rejected: failed`; *debounce* de 300 ms para mudanças em sequência; manter o último quadro no PC até o novo keyframe (sem piscar).

### Etapa B — Cliente no PC
- [ ] **F6-T03 — Atualização otimista e reconciliação** (slice `cameraState`):
  - ao alterar um controle: guardar `pending[key] = {value, sentAt}`, atualizar a UI imediatamente e enviar `SET`;
  - ao receber `STATE`: se `rev` ≤ último, **ignorar**; senão aplicar o snapshot; limpar `pending` das chaves cujo valor do celular **coincide** ou cujo `sentAt` passou de 2 s; se uma chave pendente foi **rejeitada**, voltar ao valor do celular e mostrar o toast `desktop.toast.settingRejected`;
  - uma ação **mais nova** do usuário sobre a mesma chave substitui a pendente.
- [ ] **F6-T04 — Espelho e rotação** por câmera (`presentation.*`): aplicados em `compose/` e no preview; refletidos no celular via `STATE`; a câmera virtual **não** reinicia.

### Etapa C — UIs
- [ ] **F6-T05 — Aba Câmera (PC)** (`ui/desktop.md` §2.6.2): `SegmentedControl` de câmera, resolução, fps, bitrate (`Slider` com *debounce*), bitrate automático, espelho, rotação, ajuste da saída — com `appliesTo` e estado "Aplicando…" (~1 s) nos controles que reiniciam a captura.
- [ ] **F6-T06 — Tela Transmitindo (celular):** seletor Traseira/Frontal, alternadores de microfone/espelho/rotação refletindo o `STATE`; spinner breve ao trocar câmera/resolução.

### Etapa D — Rede adaptativa
- [ ] **F6-T07 — `STATS` e bitrate adaptativo.** PC envia `STATS` a cada 1 s (fps recebido, descartes, atraso de fila). Celular aplica **AIMD** simples via `MediaCodec.setParameters`:
  - congestionamento = fila de vídeo > 150 ms **ou** descartes nos últimos 2 s → `bitrate = max(piso, bitrate × 0,8)` e *cooldown* de 3 s;
  - estável por 10 s → `bitrate = min(alvo, bitrate × 1,05)`;
  - **alvo** = valor do usuário (ou o padrão da resolução); **piso** = max(25% do alvo, 1,5 Mbps);
  - com "bitrate automático" desligado, **não** adapta.
- [ ] **F6-T08 — `REQUEST_KEYFRAME`** do PC (decodificador perdeu sincronismo) e forçado pelo celular após descartes; limitar a frequência (≤ 1 por segundo) para não saturar a rede.
- [ ] **F6-T09 — Indicadores e diagnóstico:** RTT, fps recebido, descartes e bitrate atual na UI; exportar log de diagnóstico.

### Etapa E — Persistência e ferramentas
- [ ] **F6-T10 — Preferências persistidas** (espelho, rotação, última resolução) **por dispositivo** no PC e as preferências locais no celular; migração por `schemaVersion`.
- [ ] **F6-T11 — `mock-sender --emulate-phone`:** implementa o reducer do celular (usando os **mesmos vetores**), responde `SET` com `STATE`/`rejected`, simula tempos de reconfiguração (configuráveis) e **`--chaos SEED`** (sequência aleatória reproduzível de `SET`s, quedas e reconexões).
- [ ] **F6-T12 — Testes e relatório** (§7).

## 6. Armadilhas conhecidas
- Aplicar o `STATE` fora de ordem faz a UI "voltar no tempo": sempre comparar `rev`.
- `rev` reinicia a cada sessão? **Não:** o celular mantém o `rev` enquanto o app vive; após reconexão, o primeiro `STATE` é um snapshot completo e vale como nova base.
- *Debounce* e otimismo interagem: o `pending` só vale depois que o `SET` é de fato enviado.
- Reconfigurar encoder durante a rede lenta pode demorar: não acumular reconfigurações (colapsar para a última).
- Mudar resolução altera o SPS: o decodificador do PC precisa ter sido testado com `sps-change` (Fase 3).

## 7. Plano de testes

### 7.1 Unitários
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-6.01 | Reducer (tabela): cada chave com valor válido, inválido e nos limites | Aceita/rejeita; `rejected` com o motivo certo | ✅ |
| UT-6.02 | `SET` repetido com o mesmo valor | No-op; `rev` não muda | ✅ |
| UT-6.03 | `SET` em lista com um valor inválido | Nada é aplicado (atômico) | ✅ |
| UT-6.04 | **Corrida:** sequência intercalada de `SET`s das duas origens | Estado final = efeito do **último** na ordem de chegada ao celular | ✅ |
| UT-6.05 | **Propriedade (`proptest`/Kotlin equivalente):** quaisquer intercalações de `SET`s das duas origens → PC e celular terminam com o **mesmo** estado | 10 000 casos sem divergência | ✅ |
| UT-6.06 | Reconciliação no PC: `STATE` antigo ignorado; `pending` limpo por coincidência/timeout; rejeição reverte | Conforme algoritmo | ✅ |
| UT-6.07 | Ação mais nova do usuário substitui a pendente | Só a última é considerada | ✅ |
| UT-6.08 | *Debounce* (300 ms, relógio virtual) | Uma única reconfiguração após a rajada | ✅ |
| UT-6.09 | ABR: simulação de `STATS` (queda de rede, recuperação, oscilação) | Trajetória do bitrate dentro de [piso, alvo]; *cooldown* respeitado; sem *flapping* | ✅ |
| UT-6.10 | ABR desligado | Bitrate nunca muda automaticamente | ✅ |
| UT-6.11 | Limite de `REQUEST_KEYFRAME` (≤ 1/s) | Excessos ignorados | ✅ |
| UT-6.12 | Rotação final = hint + usuário (mod 360), por câmera | Valores corretos | ✅ |
| UT-6.13 | Migração de preferências (`schemaVersion` antigo/futuro) | Migra/ignora sem *crash* | ✅ |
| UT-6.14 | RTL: aba Câmera (desabilitados com motivo, `appliesTo`, "Aplicando…", slider com *debounce*) | Conforme especificação | ✅ |
| UT-6.15 | Vetores do protocolo 1.1 (`STATE.rejected`) em Rust, TS e Kotlin | Iguais | ✅ |

### 7.2 Integração (mock `--emulate-phone`)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-6.01 | PC altera cada controle (câmera, resolução, fps, bitrate, espelho, rotação) | `STATE` volta; UI reconcilia; `compose` reflete | 🔧 |
| IT-6.02 | Rajada de 100 `SET`s em 1 s no mesmo campo | Uma reconfiguração final; sem fila crescendo | 🔧 |
| IT-6.03 | `--chaos 42` por 5 min (SETs, quedas, reconexões) | Estados convergem; sem *crash*; mesmo resultado repetindo a *seed* | 🔧 |
| IT-6.04 | `SET` durante reconexão | Aplicado após reconectar, ou descartado de forma explícita; nunca "pela metade" | 🔧 |
| IT-6.05 | `vcam-probe` aberto durante troca de resolução e de câmera | Nunca o placeholder; sem quadros fora de ordem | 🔧 |
| IT-6.06 | Troca de resolução com `sps-change` + câmera virtual | Imagem contínua; canvas de saída inalterado (1080p) | 🔧 |

### 7.3 Aceitação
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-6.01 | Trocar traseira ↔ frontal pelo PC | Imagem troca em < 1,5 s; celular mostra a câmera atual | ✋ |
| AT-6.02 | Trocar traseira ↔ frontal pelo celular | PC reflete | ✋ |
| AT-6.03 | Alterar a mesma opção nos dois lados quase ao mesmo tempo | Terminam com o **mesmo** valor | ✋ |
| AT-6.04 | Espelhar e girar no PC | Efeito imediato; celular mostra o mesmo estado | ✋ |
| AT-6.05 | Pedir resolução/fps inexistente no aparelho | `SET` rejeitado; estado corrigido; UI avisa | ✋ |
| AT-6.06 | Limitar a banda do Wi-Fi | Bitrate cai e a imagem continua fluida; ao liberar, sobe | ✋ |
| AT-6.07 | Corromper um quadro no mock | PC pede keyframe; recupera em ≤ 1 s | ✋ |
| AT-6.08 | Reconectar após queda | Estado e preferências restauram | ✋ |
| AT-6.09 | Arrastar o slider de bitrate rapidamente | Sem reinícios em cascata | ✋ |
| AT-6.10 | Câmera virtual aberta durante troca de câmera/resolução | Imagem não pisca para o placeholder | ✋ |
| AT-6.11 | Mudar espelho/rotação com o Meet/Zoom aberto | O outro lado da chamada vê a mudança sem reabrir a câmera | ✋ |
| AT-6.12 | Rodar 5 min usando só o celular para controlar (PC não tocado) | Todos os controles funcionam e o PC reflete | ✋ |
| AT-6.13 | Mudar bitrate manualmente com "automático" desligado | Valor respeitado; mostrado nos chips | ✋ |

### 7.4 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-6.01 | Câmera em uso por outro app ao pedir troca | `rejected: busy`; volta ao valor anterior; toast | ✋ |
| FT-6.02 | Encoder falha ao reconfigurar | Reverte à configuração anterior; `rejected: failed` | 🔧 |
| FT-6.03 | `STATE` fora de ordem / duplicado (mock) | Ignorados por `rev`; UI estável | ✅ |
| FT-6.04 | `SET` com chave desconhecida (protocolo futuro) | Ignorado sem derrubar a conexão | ✅ |
| FT-6.05 | Celular superaquece e reduz a qualidade | `STATE` reflete a nova resolução; UI explica ("reduzido") | ✋ |
| FT-6.06 | Troca de câmera durante rede lenta (`--slow`) | Conclui ou falha de forma limpa; sem travar | 🔧 |
| FT-6.07 | Queda do celular no meio de uma reconfiguração | PC volta a "Reconectando"; ao voltar, snapshot coerente | 🔧 |
| FT-6.08 | 50 trocas de câmera seguidas | Sem vazamento; sem *crash* | ✋ |
| FT-6.09 | `STATS` ausentes por 10 s | Celular mantém o bitrate; não degrada à toa | ✅ |

### 7.5 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-6.01 | Troca de câmera (UI → imagem nova) | ≤ 1,5 s (p95 de 20 trocas) | ✋ |
| PT-6.02 | Mudança de resolução/fps | ≤ 2 s (p95) | ✋ |
| PT-6.03 | Espelho/rotação: clique → preview | ≤ 50 ms | 🔧 |
| PT-6.04 | Convergência do ABR após limitar a banda | ≤ 5 s | ✋ |
| PT-6.05 | Sobrecarga de `STATS`/`STATE` | < 1% da banda | 🔧 |

### 7.6 Segurança
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-6.01 | `SET` com valores extremos (inteiros enormes, strings gigantes, tipos errados) | Rejeitados com `out_of_range`/`BadJson`; sem *crash* | ✅ |
| ST-6.02 | `SET` antes do `WELCOME` ou em socket de áudio | Conexão encerrada | ✅ |
| ST-6.03 | Nenhum valor de `SET` é usado em comando de shell/arquivo | Revisão de código + teste com *payloads* de injeção | ✅ |

### 7.7 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-6.01 | 30 min com `--chaos` (controles aleatórios) | Sem *crash*; estados convergem a cada 10 s; memória estável | 🔧 |
| LT-6.02 | 30 min no celular real com alternâncias a cada 30 s | Sem queda de fps; sem vazamento de encoders | ✋ |

### 7.8 Regressão
**RS-S** + AT-3/4/5 (smoke) e o **RS-F** completo antes de `v0.2.0`.

## 8. Entrega do marco M2 (`v0.2.0`)
- [ ] Fases 5 e 6 completas e testes obrigatórios aprovados; protocolo 1.1 documentado.
- [ ] README atualizado (USB, reconexão, controles).
- [ ] `docs/tests/fase-6.md` aprovado; tag `v0.2.0`.

## 9. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| Aparelho não reconfigura o encoder sem derrubar a imagem | Manter o último quadro no PC e aceitar piscada breve; documentar |
| ABR oscilando | Aumentar histerese e *cooldown*; expor parâmetros em ajustes avançados |
| Divergência de estado em casos raros | Snapshot completo periódico (a cada 10 s) como rede de segurança |
