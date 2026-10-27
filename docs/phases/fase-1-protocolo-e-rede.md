# Fase 1 — Protocolo e rede

**Marco:** M1 · **Tamanho:** M (≈ 6–9 dias) · **Depende de:** Fase 0
**Relatório:** `docs/tests/fase-1.md`

> **Execução:** para implementar, **não** use este arquivo diretamente. Siga `PROGRESS.md` e os cartões em `docs/tasks/fase-1/` (a tarefa F1-T09 foi dividida em F1-T09a e F1-T09b; a lógica de rede fica na crate `crates/lc-server`). Este arquivo é a especificação e o plano de testes da fase.

## 1. Objetivo e resultado observável
Implementar o protocolo e o **servidor do desktop** (sem UI final) e criar o **`mock-sender`**, que finge ser o celular. Ao final, todo o lado do PC pode ser desenvolvido e testado **sem aparelho**, com falhas reproduzíveis.

**Resultado observável:** o mock pareia (com aprovação), reconecta com chave, envia vídeo e áudio, e o servidor contabiliza fps/bytes; todas as falhas de protocolo são tratadas sem derrubar o servidor.

## 2. Leitura obrigatória
`docs/protocol.md` (inteiro), `docs/testing.md` §4.1 (flags do mock) e §7, `LocalCamera-PLAN.md` §8.

## 3. Escopo
**Dentro:** protocolo + vetores; `lc-protocol` (Rust) e tipos TS; servidor TCP (principal e áudio); handshake; pareamento com aprovação (por comando/evento); confiança (chaves por dispositivo); janela USB; negociação de versão; limites e proteção contra abuso; mock-sender.
**Fora:** UI de aprovação (Fase 3), mDNS (Fase 5), decodificação (Fase 3), Kotlin (Fase 2).

## 4. Entradas e saídas
| Entradas | Saídas |
|---|---|
| `docs/protocol.md` (rascunho), fixtures `hd30.h264`, `tone440.pcm` | `docs/protocol.md` final; `docs/protocol-vectors.json`; `crates/lc-protocol`; `packages/shared/src/protocol`; módulos `session/` e `trust/` do desktop; `tools/mock-sender`; `docs/tests/fase-1.md` |

## 5. Plano de trabalho

### Etapa A — Especificação e vetores
- [ ] **F1-T01 — Finalizar `docs/protocol.md` e criar os vetores.**
  - Passos: remover "rascunho"; fixar o formato exato de cada JSON (campos obrigatórios/opcionais, limites de tamanho: `HELLO` ≤ 64 KiB); criar `protocol-vectors.json` com **casos válidos e inválidos**, cada um com bytes em hex, descrição e resultado esperado (`ok{tipo,campos}` ou `erro{código}`).
  - Casos mínimos: cabeçalho válido de cada tipo; `magic` errado; `length` = 0; `length` = 8 MiB (limite) e 8 MiB+1; tipo desconhecido (deve ser pulado); JSON com campo extra; JSON sem campo obrigatório; `pts` = 0 e `u64::MAX`; quadro quebrado em 1, 2, 7, 18 e N bytes; dois quadros colados no mesmo `read`; `HELLO` de cada `auth.kind`; `WELCOME` de cada `reason`; `STATE` com `rev` fora de ordem.
  - Pronto quando: o arquivo é validado por um *schema* simples e revisado por outra leitura.

### Etapa B — Biblioteca `lc-protocol` (Rust)
- [ ] **F1-T02 — Leitor e escritor de quadros.**
  - Projeto: `FrameReader` com buffer interno e `push(&[u8]) -> Result<Vec<Frame>, ProtocolError>`; **estado** (aguardando cabeçalho / aguardando payload); nunca copiar além do necessário (`bytes::BytesMut`); limite de payload configurável (padrão 8 MiB); `ProtocolError { BadMagic, TooLarge, BadJson, ... }`.
  - Mensagens: `enum Message` com `Hello`, `Welcome`, `AudioHello`, `Pending`, `AudioTicketRequest`, `AudioTicket`, `Video`, `Audio`, `StreamConfig`, `Set`, `State`, `Ping`, `Pong`, `Stats`, `RequestKeyframe`, `Bye`, `Unknown{type, len}`.
  - JSON com `serde`: **sem** `deny_unknown_fields`; `#[serde(default)]` nos opcionais; enums de `reason` com `#[serde(other)]` para valores futuros.
  - Pronto quando: passa em todos os vetores e nos testes de propriedade (UT-1.20–UT-1.22).
- [ ] **F1-T03 — Tipos e validadores em TypeScript** (`packages/shared/src/protocol`): tipos das mensagens JSON e validadores que rodam o mesmo arquivo de vetores (parte JSON).

### Etapa C — Servidor de rede
- [ ] **F1-T04 — Listener e ciclo de vida da conexão** (`session/`)
  - Passos: `TcpListener` em `0.0.0.0:48650` (configurável); se ocupada, escolher porta livre e **expor a porta efetiva**; para cada conexão, uma *task* com `TCP_NODELAY`; **limite de conexões não autenticadas simultâneas (8)** e **timeout de 5 s** para o primeiro `HELLO`; limite do tamanho do `HELLO`.
  - Sessão única: mesmo `deviceId` **substitui** a sessão anterior (a antiga recebe `BYE`); outro `deviceId` com sessão ativa → `busy`.
  - Estados: `AwaitHello → Authenticating → (Pending) → Active → Closing`, com transições explícitas e testáveis.
- [ ] **F1-T08 — Negociação de versão:** `major` diferente → `version_mismatch` com `minSupported`; `minor` diferente aceito; registrar `app.version` e `pc.appVersion` apenas para diagnóstico.
- [ ] **F1-T10 — Keepalive e estatísticas:** `PING/PONG` a cada 2 s em cada socket; sem `PONG` por 6 s → fechar com evento `connection_lost`; contadores por stream (quadros, bytes, taxa), RTT (média móvel); agregador emitindo **a cada 500 ms** (base dos 2 Hz da UI).

### Etapa D — Confiança e pareamento
- [ ] **F1-T05 — Armazenamento e credenciais** (`trust/`)
  - Passos: `trusted_devices[]` = `{deviceId, name, keyHash(SHA-256), firstSeen, lastSeen, lastIp}`; gerar `deviceKey` com **gerador criptográfico do SO** (256 bits); **nunca** guardar a chave em claro; comparar com tempo constante (crate `subtle`); token de pareamento (128 bits) gerado na instalação e **rotacionado** após cada pareamento e em "Regenerar"; `pcId` (UUID) gerado uma vez.
  - Persistência atômica (gravar em arquivo temporário e renomear), campo `schemaVersion`, **backup e recriação** se o arquivo estiver corrompido (sem derrubar o app).
  - `auth.kind = device` com `deviceId` desconhecido ou removido → `revoked`.
- [ ] **F1-T06 — Fluxo de aprovação:** `PENDING { timeoutS:30 }`; API para a UI decidir (`approve_always`, `approve_once`, `reject`); timeout → `not_approved`; **um pedido por vez** (o segundo recebe `busy`); enquanto a UI não existe, `--auto-approve` (flag de desenvolvimento, **desligada em release**) e comandos de teste.
- [ ] **F1-T07 — Janela de aprovação USB:** `open_usb_window(15 s)`; o **primeiro** `HELLO` de `127.0.0.1` dentro da janela é aprovado sem diálogo; a janela fecha ao ser usada ou ao expirar.
- [ ] **F1-T15 — Limitação de tentativas:** após 5 autenticações falhas em 60 s de um mesmo IP, recusar novas por 5 min (sem resposta detalhada); registrar em log (sem segredos).

### Etapa E — Socket de áudio
- [ ] **F1-T09 — Plumbing do áudio:** `audioTicket` (128 bits, **uso único**, validade 10 s, preso ao `sessionId`); `AUDIO_HELLO {sessionId, ticket}` → `WELCOME`; `AUDIO_TICKET_REQUEST`/`AUDIO_TICKET`; por enquanto, contar e descartar os blocos; queda do socket de áudio **não** derruba o principal.

### Etapa F — Mock-sender
- [ ] **F1-T11 — `tools/mock-sender`** com as flags de `docs/testing.md` §4.1 marcadas "Fase 1": pareia/reconecta, reproduz `.h264` por *access unit* (usa os AUD para dividir) com *pacing* de 30 fps, envia seno de 440 Hz, injeta falhas (`--fragment`, `--bad-magic`, `--huge-length`, `--bad-json`, `--unknown-type`, `--delay-hello`, `--drop-ping`, `--slow`, `--stall`, `--kill-after`, `--reconnect`).
- [ ] **F1-T12 — Gerar os arquivos de teste** com `scripts/make-fixtures.ps1` e documentar no README do mock.
- [ ] **F1-T13 — Teste de integração base** (IT-1.01): sobe o servidor, conecta o mock, pareia, envia 10 s de vídeo + áudio e confere as contagens.
- [ ] **F1-T16 — Logs e diagnóstico:** `tracing` com níveis; cada sessão com `sessionId` nos logs; **nunca** logar token/chave/ticket completos (4 primeiros caracteres no máximo).

## 6. Armadilhas conhecidas
- Leitura TCP não preserva limites de mensagem: **todo teste do parser deve fragmentar** em todos os pontos.
- `tokio::select!` com leitura parcial pode perder bytes se o *future* for cancelado no meio; use um leitor com buffer próprio.
- Comparar token com `==` vaza informação por tempo; usar `subtle::ConstantTimeEq`.
- Não confiar no tamanho vindo da rede antes de validar o limite (alocação gigante = negação de serviço).
- Janela USB: `127.0.0.1` também pode ser **outro processo local**; por isso a janela é curta e só abre quando o usuário clicou "Conectar".
- Windows Firewall pode bloquear o primeiro `listen` em `0.0.0.0` (a regra vem na Fase 4/9).

## 7. Plano de testes

### 7.1 Unitários
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-1.01 | Todos os vetores válidos de `protocol-vectors.json` em Rust | 100% corretos | ✅ |
| UT-1.02 | Os mesmos vetores (parte JSON) em TypeScript | 100% corretos | ✅ |
| UT-1.03 | Cabeçalho little-endian: `pts` = 0 e `u64::MAX`, `length` no limite | Ida e volta idênticas (*round-trip*) | ✅ |
| UT-1.04 | `magic` inválido | `BadMagic` | ✅ |
| UT-1.05 | `length` = 8 MiB aceito; 8 MiB + 1 recusado **antes** de alocar | `TooLarge`; memória não cresce | ✅ |
| UT-1.06 | Tipo desconhecido no meio de dois quadros válidos | Pulado; os válidos são entregues | ✅ |
| UT-1.07 | JSON com campos extras | Aceito, extras ignorados | ✅ |
| UT-1.08 | JSON sem campo obrigatório / tipo errado | `BadJson` | ✅ |
| UT-1.09 | `reason` desconhecido em `WELCOME` | Interpretado como "outro", sem erro | ✅ |
| UT-1.10 | Entrada fragmentada em **cada** posição possível (1..N bytes) para cada vetor | Mesmo resultado que sem fragmentar | ✅ |
| UT-1.11 | Dois/três quadros no mesmo `push` | Todos entregues em ordem | ✅ |
| UT-1.12 | Geração de `deviceKey`/token: tamanho e formato (hex 64/32) e unicidade em 10 000 amostras | Sem colisões; só hex | ✅ |
| UT-1.13 | Armazenamento guarda **só o hash** da chave | Arquivo não contém a chave em claro | ✅ |
| UT-1.14 | Comparação de segredos usa função de tempo constante (teste de API: entradas de tamanhos diferentes não causam *panic*) | Sem *panic*; resultado falso | ✅ |
| UT-1.15 | Máquina de estados da conexão (tabela de transições) | Transições inválidas impossíveis | ✅ |
| UT-1.16 | Aprovação: `approve_always` grava; `approve_once` não grava; `reject`; timeout (tempo virtual) | Estados e `WELCOME` corretos | ✅ |
| UT-1.17 | Rotação do token após pareamento; QR antigo recusado | `bad_token` | ✅ |
| UT-1.18 | Janela USB: abre, expira em 15 s (tempo virtual), uso único, só `127.0.0.1` | Conforme especificação | ✅ |
| UT-1.19 | Ticket de áudio: uso único, expira em 10 s, preso ao `sessionId` | Reuso/expirado/outra sessão → `bad_audio_ticket` | ✅ |
| UT-1.20 | **Propriedade:** para qualquer sequência de quadros válidos e qualquer particionamento aleatório dos bytes, o leitor produz a mesma sequência | `proptest` com ≥ 10 000 casos | ✅ |
| UT-1.21 | **Propriedade:** bytes aleatórios nunca causam *panic* nem alocação > limite | `proptest` | ✅ |
| UT-1.22 | **Propriedade:** *round-trip* de qualquer `Message` válida | `proptest` | ✅ |
| UT-1.23 | Negociação de versão (matriz major/minor: 1.0×1.0, 1.0×1.3, 1.3×1.0, 1.0×2.0, 0.9×1.0) | Aceita/recusa conforme §2 do protocolo | ✅ |
| UT-1.24 | Limitador de tentativas: 5 falhas/60 s → bloqueio por 5 min; sucesso zera; outro IP não afetado | Conforme regra | ✅ |
| UT-1.25 | Armazenamento corrompido (JSON truncado, vazio, `schemaVersion` futuro) | Backup do arquivo e recriação; app não cai | ✅ |
| UT-1.26 | Escrita atômica: interromper entre gravar e renomear (simulado) | Arquivo anterior permanece íntegro | ✅ |

### 7.2 Integração (servidor + mock)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-1.01 | Pareia, envia 10 s de vídeo (`hd30.h264`) e 10 s de áudio | ≈ 300 quadros e ≈ 500 blocos contados; sem erros | ✅ |
| IT-1.02 | Reconexão com `--device-key` | Aceita sem aprovação | ✅ |
| IT-1.03 | 20 reconexões seguidas (`--reconnect 20`) | Sem vazamento de *tasks*/descritores; contadores corretos | ✅ |
| IT-1.04 | Mock com `--fragment 1` (1 byte por escrita) durante 5 s | Mesmos totais que sem fragmentar | ✅ |
| IT-1.05 | Substituição de sessão (mesmo `deviceId`) | Antiga recebe `BYE`; nova ativa | ✅ |
| IT-1.06 | Dois mocks, `deviceId` diferentes | Segundo recebe `busy` | ✅ |
| IT-1.07 | Socket de áudio cai e reabre com novo ticket | Principal intacto; áudio retoma | ✅ |
| IT-1.08 | Estatísticas agregadas | Emitidas a cada 500 ms ± 50 ms | ✅ |

### 7.3 Aceitação (automatizados via mock; mantidos com os IDs originais)
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-1.01 | Vetores de protocolo em Rust e TS | 100% passando | ✅ |
| AT-1.02 | Mock com token correto e `--auto-approve` | Aceito; `deviceKey` entregue; token rotacionado | ✅ |
| AT-1.03 | Mock com token errado | `bad_token` e socket fechado | ✅ |
| AT-1.04 | Reconectar o mock com `--device-key` | Aceito sem aprovação | ✅ |
| AT-1.05 | Revogar o dispositivo e reconectar | `revoked` | ✅ |
| AT-1.06 | Sem resposta de aprovação por 30 s | `not_approved` | ✅ |
| AT-1.07 | Mock com `--fake-version 2.0` | `version_mismatch` com `minSupported` | ✅ |
| AT-1.08 | Segundo mock com outro `deviceId` | `busy` | ✅ |
| AT-1.09 | Mock com `--audio` | Socket de áudio autenticado; blocos contados; ticket reutilizado recusado | ✅ |
| AT-1.10 | Cortar `PONG` por 6 s (`--no-pong`) | Servidor encerra a sessão e emite `connection_lost` | ✅ |
| AT-1.11 | Quadro com `length` > 8 MiB (`--huge-length`) | `protocol_error` | ✅ |
| AT-1.12 | Mensagem de tipo desconhecido no meio do stream | Pulada e logada; conexão segue | ✅ |
| AT-1.13 | Janela USB aberta + conexão de `127.0.0.1` | Aprovado sem diálogo; fora da janela, fluxo normal | ✅ |
| AT-1.14 | Mock `--bad-json` no `HELLO` | `protocol_error`; servidor continua aceitando outros | ✅ |
| AT-1.15 | Reiniciar o servidor com dispositivos já pareados | Dispositivos continuam reconhecidos | ✅ |

### 7.4 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-1.01 | Mock conecta e **não envia** `HELLO` (*slowloris*) | Fechado em 5 s; recursos liberados | ✅ |
| FT-1.02 | 200 conexões simultâneas sem autenticar | Limite de 8 respeitado; as demais recusadas/fechadas; servidor responsivo | ✅ |
| FT-1.03 | Mock morto no meio de um quadro (`--kill-after`) | Sessão encerrada sem *panic*; estado limpo | ✅ |
| FT-1.04 | `HELLO` gigante (> 64 KiB) | Recusado antes de alocar | ✅ |
| FT-1.05 | Porta 48650 ocupada por outro processo | Servidor usa porta alternativa e expõe a efetiva | ✅ |
| FT-1.06 | Conexão "meio aberta" (sem `FIN`, rede cortada) | Detectada pelo keepalive em ≤ 6 s | ✅ |
| FT-1.07 | Tempestade de reconexões (50 em 5 s) | Limitador atua; CPU baixa | ✅ |
| FT-1.08 | Aprovação pendente e o mock desconecta | Pedido cancelado; UI/evento informado; sem pendência fantasma | ✅ |
| FT-1.09 | Arquivo de confiança apagado com o app aberto | Próxima conexão pede novo pareamento; sem *crash* | ✅ |
| FT-1.10 | `AUDIO_HELLO` antes do `WELCOME` do principal | Recusado (`bad_audio_ticket`) | ✅ |

### 7.5 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-1.01 | Vazão do parser (leitura de 1 GB sintético em blocos de 64 KiB) | ≥ 500 MB/s em uma thread | 🔧 |
| PT-1.02 | 50 handshakes por segundo por 10 s | Sem falhas; p95 do handshake ≤ 20 ms (rede local) | 🔧 |
| PT-1.03 | Stream de 8 Mbps + áudio por 5 min | CPU do servidor ≤ 3% de um núcleo; memória estável | 🔧 |

### 7.6 Segurança
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-1.01 | Procurar token/chave/ticket completos em logs e no arquivo de confiança | Não aparecem | ✅ |
| ST-1.02 | Força bruta de token (6 tentativas/min) | Bloqueio do IP; resposta uniforme | ✅ |
| ST-1.03 | Reuso de `audioTicket` e de `deviceKey` revogada | Recusados | ✅ |
| ST-1.04 | Mensagens (`SET`, `VIDEO`…) antes do `WELCOME` | Conexão encerrada | ✅ |
| ST-1.05 | `cargo audit` e `cargo deny` (licenças) | Sem vulnerabilidades conhecidas; licenças permitidas | 🔧 |
| ST-1.06 | `--auto-approve` em build de release | **Ausente** (não compila ou é ignorado) | ✅ |

### 7.7 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-1.01 | Mock a 30 fps/8 Mbps + áudio por **30 min** | Memória estável (≤ 5 MB de crescimento); sem *handles* vazando; contadores consistentes | 🔧 |
| LT-1.02 | 500 reconexões | Sem vazamento | 🔧 |

### 7.8 Regressão
Reexecutar **RS-S** ao final. Os vetores passam a fazer parte do CI para sempre.

## 8. Checklist de saída
- [ ] UT/IT/AT/FT/ST ✅ todos aprovados; PT e LT dentro das metas; relatório preenchido.
- [ ] `docs/protocol.md` final e sincronizado com os vetores e os tipos TS.
- [ ] Mock-sender documentado (README com todos os *flags*).
- [ ] `pnpm ci` verde.

## 9. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| Parser com falha sutil em fragmentação | Testes de propriedade + `cargo fuzz` opcional |
| Aprovação concorrente gerando estado inconsistente | Máquina de estados única e testada por tabela |
| Escopo crescer para a UI | Manter UI fora; usar comandos de teste e `--auto-approve` |
