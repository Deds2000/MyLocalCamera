# Fase 2 — Mobile: transmissão mínima

**Marco:** M1 · **Tamanho:** G (≈ 10–15 dias) · **Depende de:** Fase 0 (S3), Fase 1
**Relatório:** `docs/tests/fase-2.md`

## 1. Objetivo e resultado observável
Um celular real **pareia** com o desktop (QR → aprovação), **reconecta com a chave** e **envia vídeo H.264 1080p30** (câmera traseira) por Wi-Fi. O desktop ainda só mostra estatísticas (a UI vem na Fase 3).

## 2. Leitura obrigatória
`docs/protocol.md` (§3–§6, §10–§12), `docs/ui-spec.md` (design system), **`docs/ui/mobile.md`** (telas, navegação, erros), `docs/ui/copy.md`, `docs/spikes/S3.md` (backend de câmera), `docs/testing.md`.

## 3. Escopo
**Dentro:** telas Permissões, Home, Scan, Manual, Conectando, Aguardando, Transmitindo (básica), PCs conhecidos e erros definitivos; deep link; plugin Kotlin (capacidades, câmera única, encoder, conexão principal, pareamento, fila de envio); serviço em primeiro plano básico; catálogo de componentes.
**Fora:** câmera dupla (7), áudio (8), mDNS e USB (5), controles remotos (6), configurações de segundo plano completas (9).

## 4. Entradas e saídas
| Entradas | Saídas |
|---|---|
| Protocolo e vetores (Fase 1), mock/desktop servidor, veredito do S3 | App Android funcional; `ProtocolCodec.kt` passando nos vetores; textos em `packages/shared/src/i18n` e `strings.xml`; capturas em `docs/ui/screens/mobile/`; `docs/devices.md` (primeira versão); `docs/tests/fase-2.md` |

## 5. Plano de trabalho

### Etapa A — Casca do app e navegação
- [ ] **F2-T01 — Capacitor e deep link.** `capacitor.config.ts`; intent-filter `localcamera://connect`; `@capacitor/app` com `appUrlOpen`; `launchMode` adequado para receber a URI com o app já aberto; travar a UI em retrato; barra de status/navegação conforme `ui/mobile.md` §3.13.
- [ ] **F2-T03 — Telas e roteamento** exatamente como `docs/ui/mobile.md`: Permissões (§3.3), Home (§3.4), Scan (§3.5), Manual (§3.6), Conectando e Aguardando (§3.7), Transmitindo (§3.8, sem a sobreposição de reconexão por mDNS — Fase 5), PCs conhecidos (§3.9), erros (§3.10). Roteamento **derivado do estado** (§3.1); botão voltar por tela.
- [ ] **F2-T03a — Tokens e componentes próprios** (`CountdownRing`, `BigToggle`, `LiveRing`, `StatTile`, `SegmentedControl`, `StatusChip`) conforme `ui-spec` §1.10; `reduce-glass` automático em `Streaming`.
- [ ] **F2-T03b — Textos:** copiar `common.*` e `mobile.*` de `ui/copy.md` para os JSON de i18n e para `strings.xml` (en e pt-rBR).
- [ ] **F2-T03c — Catálogo (`?catalog`, só em dev)** com todos os componentes e estados; capturas em `docs/ui/screens/mobile/`.

### Etapa B — Protocolo e armazenamento no Kotlin
- [ ] **F2-T04 — `ProtocolCodec.kt`:** leitor incremental (mesmo desenho do Rust: buffer + estados), escritor, JSON das mensagens; limite de payload; **tipos desconhecidos pulados**. Passa em **todos** os vetores de `protocol-vectors.json`.
- [ ] **F2-T05 — `PairingStore.kt`:** `deviceId` (UUID estável, gerado uma vez), lista de PCs conhecidos (`pcId`, nome, hosts, porta, último acesso) e `deviceKey` em armazenamento privado do app (Keystore; **confirmar a API recomendada atual**, algumas bibliotecas de armazenamento seguro foram descontinuadas). **Excluído do backup** (manifesto + regras de backup).
- [ ] **F2-T05a — Parser da URI** `localcamera://connect`: valida `v`, `hosts` (IPv4, até 4), `port` (1–65535), `token` (32 hex), `pc`, `name`; ignora parâmetros desconhecidos; `v` maior que o suportado → "atualize o app".

### Etapa C — Captura e codificação
- [ ] **F2-T06 — `CapabilitiesProbe`:** câmeras (id, facing, modos resolução × fps), `concurrent` (real, do S3), encoders H.264 e perfis → `HELLO.caps`.
- [ ] **F2-T07 — `CameraBackend` + implementação escolhida no S3.** Interface: `start(config, outputSurface)`, `stop()`, `switchTo(facing)`, `onRotationHint`. A UI/Session **nunca** importa CameraX/Camera2 diretamente.
- [ ] **F2-T08 — `VideoEncoder.kt`** (MediaCodec H.264)
  - Configuração: `COLOR_FormatSurface`; **CBR**; bitrate 8 Mbps; keyframe a cada 1 s; `KEY_LOW_LATENCY` (quando suportado) e prioridade de tempo real; sem B-frames; perfil High com *fallback*.
  - Saída: tratar `BUFFER_FLAG_CODEC_CONFIG` (guardar SPS/PPS) e **anexar SPS/PPS a cada keyframe** se o encoder não o fizer; `flags` do quadro (bit0 keyframe, bit1 com parameter sets); `ptsUs` derivado do relógio monotônico, mesmo relógio do áudio futuro.
  - API: `requestKeyframe()` (parâmetro de sincronismo do MediaCodec), `setBitrate()`.
  - Registrar encoder, perfil e nível escolhidos (log + diagnóstico).

### Etapa D — Rede e sessão
- [ ] **F2-T09 — `PrimaryConnection.kt`:** tenta todos os `hosts` **em paralelo** (2 s); `HELLO` (`pair` ou `device`); trata `PENDING`/`WELCOME`/`reason`; guarda `deviceKey` recebida; envia `STATE`, `STREAM_CONFIG` e o primeiro keyframe; `PING/PONG`; reconexão com *backoff* 1-2-4-8 s (relógio injetável para teste).
- [ ] **F2-T10 — `SendQueue.kt`:** duas faixas (controle > vídeo); escritor único em coroutine; limite de fila por **tempo** (> 150 ms): descartar quadros não-chave antigos, **nunca** descartar o keyframe mais recente, pedir keyframe; `TCP_NODELAY`; `SO_SNDBUF` reduzido (valor decidido por medição).
- [ ] **F2-T11 — `StateReducer.kt`:** estado mínimo (câmera traseira, 1080p30, bitrate), `rev`, `STATE` ao conectar.
- [ ] **F2-T12 — `StreamingService`** (`LifecycleService`, foreground, tipos `camera|microphone`; permissões de manifesto: `CAMERA`, `RECORD_AUDIO`, `INTERNET`, `ACCESS_NETWORK_STATE`, `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_CAMERA`, `FOREGROUND_SERVICE_MICROPHONE`, `POST_NOTIFICATIONS`, `WAKE_LOCK`; confirmar na documentação os requisitos por versão do Android). Iniciar **com o app visível**; notificação com ação "Parar" e textos em `strings.xml`.
- [ ] **F2-T13 — `LocalCameraPlugin.kt`:** `getCapabilities`, `connect({hosts, port, token | pcId})`, `disconnect`, `getState`; eventos `connectionChanged`, `stateChanged`, `stats` (≤ 2 Hz), `error` (**códigos**).
- [ ] **F2-T16 — Rede do Android:** garantir que o socket use a rede Wi-Fi quando o aparelho tem dados móveis e o Wi-Fi não tem internet (testar `ConnectivityManager`/vinculação de rede; **confirmar** a abordagem) e documentar o comportamento com **VPN ativa**.

### Etapa E — Qualidade
- [ ] **F2-T14 — Testes** (ver §7): unitários Kotlin, de UI (RTL) e instrumentados.
- [ ] **F2-T15 — Relatório e matriz de aparelhos** (`docs/devices.md`): modelo, Android, encoder, resultados.

## 6. Armadilhas conhecidas
- O Android restringe iniciar uso de câmera/microfone a partir do segundo plano: iniciar a transmissão **sempre** com o app visível.
- Alguns encoders ignoram `KEY_PREPEND_HEADER_TO_SYNC_FRAMES`; confira se cada keyframe carrega SPS/PPS.
- O primeiro quadro de saída costuma ser `CODEC_CONFIG` (não é vídeo): não enviar como `VIDEO`.
- `rotationHint` no `STREAM_CONFIG` precisa mudar quando o aparelho for girado (a UI fica em retrato, mas o vídeo reflete a orientação do sensor).
- Wi-Fi com "isolamento de clientes" ou VPN no celular podem impedir o acesso ao PC (mensagem clara na UI).
- Mudar de rede Wi-Fi com o app aberto invalida os `hosts`.
- Poupança de bateria agressiva pode matar o serviço (Fase 9 trata; aqui registrar o comportamento).

## 7. Plano de testes

### 7.1 Unitários (JVM e RTL)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-2.01 | `ProtocolCodec` nos vetores do protocolo | 100% iguais ao Rust | ✅ |
| UT-2.02 | Codec com entrada fragmentada em toda posição e com tipos desconhecidos | Mesmos resultados | ✅ |
| UT-2.03 | Parser da URI: válida, sem `token`, `token` curto, `port` 0/70000, `hosts` vazio/IPv6/6 itens, parâmetros extras, `v=2` | Aceita/recusa conforme §3.5/§3.6 de `ui/mobile.md` e protocolo §4 | ✅ |
| UT-2.04 | `SendQueue`: controle passa na frente do vídeo; descarte por tempo; keyframe mais recente preservado; pedido de keyframe | Conforme regras (relógio virtual) | ✅ |
| UT-2.05 | *Backoff* de reconexão 1-2-4-8-8… e *reset* após sucesso | Sequência correta (relógio virtual) | ✅ |
| UT-2.06 | `StateReducer`: estado inicial, `rev` incrementa, `STATE` serializado | Conforme protocolo §9 | ✅ |
| UT-2.07 | `PairingStore`: grava/lê PC e chave; esquecer apaga a chave; arquivo corrompido | Sem *crash*; chave não aparece em texto claro nos *logs* | ✅ |
| UT-2.08 | Seleção de tela a partir de (estado de conexão, permissões, erro) — tabela completa de `ui/mobile.md` §3.1/§3.10 | Tela correta para cada combinação | ✅ |
| UT-2.09 | Mapeamento de `reason` → tela/mensagem (todos os códigos) | Chave de i18n correta; nenhum texto vindo da rede | ✅ |
| UT-2.10 | Cobertura de i18n: toda chave usada existe em pt-BR e en | Sem faltas | ✅ |
| UT-2.11 | RTL: Home (com/sem PC conhecido, sem rede, sem permissões); Scan (QR inválido); Manual (validações); Aguardando (contagem); Transmitindo (tiles, aviso de rede lenta) | Textos, estados e acessibilidade corretos | ✅ |
| UT-2.12 | Botão voltar por tela (lógica) | Conforme tabela `ui/mobile.md` §3.1 | ✅ |
| UT-2.13 | Visual: capturas do `?catalog` (Playwright) | Sem diferenças não aprovadas | 🔧 |
| UT-2.14 | Acessibilidade automática (axe) nas telas do catálogo | Sem violações sérias | 🔧 |

### 7.2 Integração (instrumentados e com o desktop)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-2.01 | `androidTest`: encoder 1080p30 por 10 s em memória | Saída Annex-B; **cada keyframe com SPS+PPS**; intervalo de keyframe ≈ 1 s (± 100 ms) | 🔧 |
| IT-2.02 | `androidTest`: bitrate medido em 10 s | 8 Mbps ± 15% | 🔧 |
| IT-2.03 | `androidTest`: `requestKeyframe()` | Keyframe em ≤ 300 ms | 🔧 |
| IT-2.04 | `androidTest`: `CapabilitiesProbe` | Lista de modos coerente com o aparelho; `concurrent` igual ao do S3 | 🔧 |
| IT-2.05 | Celular → servidor do desktop com `--auto-approve` | Pareia e transmite; o servidor loga ≈ 30 fps | 🔧 |
| IT-2.06 | Celular ↔ servidor: reconexão com chave | Sem aprovação | 🔧 |
| IT-2.07 | Serviço em primeiro plano inicia/para com a notificação correta | Notificação em pt-BR e en | ✋ |

### 7.3 Aceitação em aparelho real
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-2.01 | Rodar os vetores do protocolo no Kotlin | 100% | ✅ |
| AT-2.02 | Escanear o QR do desktop | "Aguardando aprovação" → após aprovar, "Transmitindo" | ✋ |
| AT-2.03 | Fechar e reabrir o app, tocar em Reconectar (mesmos IPs) | Conecta com a chave, **sem** novo diálogo | ✋ |
| AT-2.04 | Recusar a aprovação no PC | Mensagem de "não aprovado" | ✋ |
| AT-2.05 | Transmitir 10 min em 1080p30 | Desktop ≈ 30 fps; sem desconexão; sem aquecimento anormal | ✋ |
| AT-2.06 | Desligar o Wi-Fi do PC por 5 s e religar | Celular reconecta sozinho com *backoff* | ✋ |
| AT-2.07 | Limitar a banda durante a transmissão | Quadros descartados, keyframe forçado, bitrate reduz; `PING/PONG` seguem | ✋ |
| AT-2.08 | Remover o celular da lista de confiáveis e reconectar | Recebe `revoked` e apaga a chave | ✋ |
| AT-2.09 | Versão incompatível simulada | "Atualize o app do PC" com as duas versões | ✋ |
| AT-2.10 | Escanear o QR com a câmera nativa do Android | Abre o LocalCamera já com a URI | ✋ |
| AT-2.11 | Checklist de `ui-spec` §8 em todas as telas (fonte a 130% e 200%, pt-BR e en, TalkBack no fluxo Home→Scan→Transmitindo→Parar) | Sem cortes; sem texto fixo; alvos ≥ 44 px | ✋ |
| AT-2.12 | Botão voltar em cada tela | Conforme `ui/mobile.md` §3.1 | ✋ |
| AT-2.13 | Entrada manual (IP, porta, código) e **Colar link** | Conecta; validações exibidas inline | ✋ |
| AT-2.14 | Girar o aparelho e apoiá-lo deitado/em pé durante a transmissão | `rotationHint` correto no `STREAM_CONFIG`; UI permanece em retrato | ✋ |
| AT-2.15 | Lista "Meus PCs": conectar e esquecer | Esquecer apaga a chave; próxima conexão exige QR | ✋ |

### 7.4 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-2.01 | Modo avião durante a transmissão e depois desligar | Entra em reconexão e volta sozinho | ✋ |
| FT-2.02 | Mudar o celular de rede Wi-Fi | Reconexão falha com `hosts` antigos → mensagem para escanear de novo | ✋ |
| FT-2.03 | Encerrar o desktop durante a transmissão | Celular mostra "Reconectando…" e depois a falha, sem travar | ✋ |
| FT-2.04 | Receber uma chamada telefônica durante a transmissão | Transmissão pausa/encerra de forma limpa, com aviso; app não trava | ✋ |
| FT-2.05 | Outro app abre a câmera | Erro de câmera tratado; mensagem; reconexão manual | ✋ |
| FT-2.06 | Revogar a permissão de câmera nas configurações com o app aberto | Transmissão para; tela de permissão | ✋ |
| FT-2.07 | Matar o app (arrastar da lista) com o serviço ativo | Serviço encerra; notificação some; PC mostra desconexão | ✋ |
| FT-2.08 | Economia de bateria ligada | Comportamento registrado; sem *crash* | ✋ |
| FT-2.09 | Pouca memória (abrir apps pesados) | Sem *crash*; reconexão ao voltar | ✋ |
| FT-2.10 | VPN ativa no celular | Mensagem clara se o PC for inalcançável; funciona com VPN *split* | ✋ |
| FT-2.11 | Dados móveis ligados e Wi-Fi sem internet | Conecta ao PC pela rede local | ✋ |
| FT-2.12 | Servidor recusa com cada `reason` (via mock-servidor/desktop) | A tela correta para cada código | ✅ |
| FT-2.13 | `STREAM_CONFIG` + keyframe perdidos (simulado) | PC pede keyframe; celular responde em ≤ 300 ms | ✅ |
| FT-2.14 | Escanear 20 QR codes inválidos seguidos | Sem *crash* nem vazamento; faixa de erro some após 3 s | ✋ |

### 7.5 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-2.01 | fps efetivo em 10 min (1080p30) | ≥ 29,5 | 🔧 |
| PT-2.02 | Latência câmera → quadro codificado | ≤ 80 ms (p95); registrar o valor | 🔧 |
| PT-2.03 | Uso de CPU/temperatura do celular após 10 min | Registrar; sem *throttling* térmico visível | ✋ |
| PT-2.04 | Bateria consumida em 10 min | Registrar (linha de base para a Fase 9) | ✋ |
| PT-2.05 | Tempo de partida a frio do app | ≤ 1,5 s (registrar) | ✋ |
| PT-2.06 | Tempo QR → "Transmitindo" (com aprovação imediata) | ≤ 4 s | ✋ |

### 7.6 Segurança
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-2.01 | `adb backup`/extração de dados do app | `deviceKey` **não** presente | ✋ |
| ST-2.02 | `logcat` durante pareamento e deep link | Token/chave **não** aparecem (no máximo 4 caracteres) | ✋ |
| ST-2.03 | Manifesto: componentes exportados | Só `MainActivity` (e o intent-filter do esquema) | ✅ |
| ST-2.04 | Deep link malicioso (URI com `hosts` estranhos, `port` absurda, `name` gigante) | Rejeitado/limitado sem *crash*; nunca conecta sem confirmação do usuário | ✅ |
| ST-2.05 | WebView em release | Depuração remota desativada | ✋ |

### 7.7 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-2.01 | **30 min** transmitindo | fps estável; memória do app estável; temperatura registrada; sem desconexão | ✋ |
| LT-2.02 | 50 ciclos de conectar/parar | Sem vazamento; sem *crash* | ✋ |

### 7.8 Regressão
Reexecutar **RS-S** e **AT-1.xx** (servidor) após qualquer mudança no protocolo.

## 8. Checklist de saída
- [ ] Todos os testes obrigatórios aprovados; `docs/devices.md` com ≥ 1 aparelho (idealmente 2).
- [ ] Capturas do catálogo em `docs/ui/screens/mobile/`.
- [ ] `docs/tests/fase-2.md` preenchido.

## 9. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| Plugin de QR incompatível com Capacitor 8.5 | Isolar atrás de wrapper; alternativa: scanner nativo próprio com CameraX + ML Kit |
| Encoder do aparelho com comportamento fora do padrão | Fallback de perfil; testar um segundo aparelho |
| Armazenamento seguro com API descontinuada | Usar Keystore diretamente com criptografia própria mínima |
