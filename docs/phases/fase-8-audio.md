# Fase 8 — Áudio: spike S4 e microfone virtual (marco M4)

**Marco:** M4 → tag `v0.4.0` · **Tamanho:** G (≈ 15–25 dias, dependendo do S4) · **Depende de:** Fase 6 (**não comece antes de a câmera funcionar**)
**Relatório:** `docs/tests/fase-8.md`

## 1. Objetivo e resultado observável
O microfone do celular aparece no Windows como **"LocalCamera Microphone"** (ou equivalente) e funciona em Discord, OBS, Zoom e gravadores, **sincronizado com o vídeo**, sem estalos.

**Resultado observável:** o `audio-probe` captura o endpoint e confirma o tom de 440 Hz sem descontinuidades; Discord/OBS/Zoom usam o microfone; ajuste de atraso alinha áudio e vídeo.

## 2. Ordem de trabalho
1. **S4 — porta de decisão** (como obter o microfone virtual).
2. **8a — Áudio ponta a ponta com saída interina** (não depende do driver).
3. **8b — Microfone virtual** conforme a decisão do S4.

## 3. Leitura obrigatória
`docs/protocol.md` §3, §10, §11; `docs/testing.md` §4.2 (`audio-probe`); páginas oficiais: exemplo **SYSVAD** (Windows-driver-samples/audio/sysvad), exemplos **ACX** (Windows-driver-samples/audio/Acx), opção de boot `TESTSIGNING`, e "Driver compatibility with memory integrity and VBS".

## 4. Escopo
**Dentro:** S4; captura, transporte e *jitter buffer*; saída interina; ganho, atraso, medidor; microfone virtual (reuso, fork ou driver próprio); `audio-probe`; instalação/reversão; diagnóstico de ambiente.
**Fora:** compressão (Opus), cancelamento de eco/ruído.

---

## 5. Spike S4 — Como obter o microfone virtual

**Por que agora:** é o item mais arriscado do projeto; só vale o esforço depois que a webcam funciona.

### S4a — Pesquisa de soluções existentes (≈ 1 dia)
Para **cada candidata**: licença, manutenção recente, se expõe **endpoint de captura**, como aceita dados de um app (loopback render→captura ou IOCTL), **como é assinada** e se instala **sem modo de teste**, compatibilidade com HVCI e tamanho do código.

Candidatas conhecidas (**confirmar URL, licença e estado atual**):
- **Virtual-Audio-Driver** (organização *VirtualDrivers* no GitHub): alto-falante e microfone virtuais para Windows 10/11, MIT segundo a organização, com releases assinadas via SignPath. Se o microfone funcionar como *loopback* do alto-falante virtual, o app só precisa **tocar o áudio do celular no alto-falante virtual** (WASAPI) — **sem driver próprio**.
- **Exemplo ACX AudioCodec** da Microsoft (MIT): já foi usado por outros projetos como base de microfone virtual (dispositivo enumerado na raiz, só circuito de captura, PCM injetado por IOCTL).
- **SYSVAD** da Microsoft (MIT): exemplo completo e mais complexo.
- Soluções proprietárias (cabos de áudio virtuais populares): **fogem do critério de código aberto**; só como último recurso, fora do repositório.

### S4b — Viabilidade técnica (conforme o caminho)
- Compilar/instalar a candidata numa **VM ou PC de teste** e confirmar que o endpoint de captura aparece em Configurações → Som.
- **Reuso sem modificar:** testar o caminho "tocar no alto-falante virtual" com o `audio-probe` capturando o microfone.
- **Fork:** renomear para "LocalCamera Microphone", remover o circuito de renderização, expor IOCTL de escrita.
- **Driver próprio** (ACX ou SYSVAD): um único endpoint de captura 48 kHz/16 bits/mono, **buffer circular** preenchido por IOCTL e **um único escritor** (abertura exclusiva; dois escritores intercalam amostras e soam como chiado).
- **Arquitetura testável:** manter a lógica (buffer circular, conversão de formato, *underrun*) numa **biblioteca pura**, testável em modo usuário (§8.2), separada do código de kernel.

### S4c — Ambiente e riscos do sistema (obrigatório documentar)
- **Modo de teste** (`bcdedit /set testsigning on`) exige **Secure Boot desligado**; o Windows exibe marca d'água.
- **Memory Integrity (HVCI):** com ela ligada, um binário não assinado não carrega nem com modo de teste; é preciso **assinar com certificado de teste** e o driver deve ser **compatível com HVCI** (teste de prontidão do HLK) — ou desligar a opção durante o desenvolvimento.
- **Jogos com anti-cheat em modo kernel** podem recusar rodar com Secure Boot desligado ou modo de teste ativo (**confirmar nos jogos que você usa**). Mitigações: **entrada de boot separada** para o modo de teste; ou driver de terceiros **já assinado**.
- Nunca instalar drivers experimentais na máquina principal. Usar VM/PC de teste com WinDbg/ETW; anotar o procedimento de **reversão**.

### Saída do S4
`docs/spikes/S4.md` + ADR com **uma** decisão: (1) **reutilizar** driver existente; (2) **fork**; (3) **driver próprio**; (4) **sem driver no MVP** (só saída interina), com o motivo. Registrar as consequências (modo de teste? HVCI? anti-cheat?). **Se for (3) ou (4), reavaliar o tamanho da fase com o usuário.**

---

## 6. Plano de trabalho

### Etapa A — Áudio de ponta a ponta (8a)
- [ ] **F8-T01 — `AudioCapture.kt`:** `AudioRecord` 48 kHz mono PCM 16, buffer ≥ `getMinBufferSize`, leitura em blocos de **10 ms (480 amostras)** em *thread* de prioridade de áudio; fonte configurável (padrão `MIC`; opção `VOICE_COMMUNICATION`); `ptsUs` do **mesmo relógio** do vídeo; respeita `audio.enabled`; ao mutar, **parar a captura** (indicador de privacidade some); tratar `ERROR_DEAD_OBJECT`/falhas reiniciando; detectar **microfone bloqueado pelo sistema** (silêncio absoluto por > 2 s) → aviso.
- [ ] **F8-T02 — `AudioConnection.kt`:** abre o socket de áudio **depois** do `WELCOME` (com `audioTicket`); `PING/PONG`; reabertura via `AUDIO_TICKET_REQUEST`; fila **própria** (descarta os blocos mais antigos acima de ~200 ms); `TCP_NODELAY`; não depende do socket de vídeo.
- [ ] **F8-T03 — Desktop `audio/`:** **jitter buffer** com alvo de 60 ms (mín. 40, máx. 120) e adaptação; **correção de deriva** por reamostragem fina (ajuste de ±0,5% conforme a tendência de ocupação do buffer; biblioteca de reamostragem a confirmar); *underrun* → silêncio (e contador); *overrun* → descarta/acelera; ganho com limitador suave; medidor RMS/pico em dBFS (≤ 20 Hz).
- [ ] **F8-T04 — Saída interina:** tocar em um dispositivo de saída escolhido (WASAPI, crate `cpal` ou equivalente; modo compartilhado, período ~10 ms). **Se o S4 apontar para um driver com alto-falante virtual, este caminho vira a solução final.**
- [ ] **F8-T05 — UI:** `audio.enabled`, medidor, ganho, atraso, saída interina (`ui/desktop.md` §2.6.3; celular §3.8); erros de permissão negada.
- [ ] **F8-T06 — Sincronia A/V:** estimar o offset de relógio a partir dos `ptsUs`; slider de atraso (0–500 ms) implementado como atraso extra no áudio; documentar o procedimento de calibração com palmas.
- [ ] **F8-T07 — Permissões e serviço:** microfone no serviço em primeiro plano (tipo `microphone`); iniciar com o app visível.

### Etapa B — Microfone virtual (8b), conforme o S4
- [ ] **F8-T08 — Integrar a solução escolhida** em `driver/audio/` e `scripts/`: instalação (INF + `pnputil`; criação do dispositivo raiz com a ferramenta atual do WDK — **confirmar**), desinstalação e diagnóstico (`scripts/mic-setup.ps1` com `-Uninstall`).
- [ ] **F8-T09 — Se fork/próprio:** IOCTL de escrita, buffer circular, **escritor único** (abertura exclusiva), formato 48 kHz/16 bits/mono (o Windows converte), *underrun* → silêncio, validação de **todos** os tamanhos de buffer do IOCTL, nenhuma divulgação de memória do kernel.
- [ ] **F8-T10 — Rodapé:** **Microfone virtual** — ausente / modo de teste necessário / HVCI bloqueando / ativo, com instruções por estado (`ui/desktop.md` §2.7).
- [ ] **F8-T11 — Assinatura e reversão:** certificado de teste (se aplicável) + script de instalação do certificado; documentar a reversão (`bcdedit /set testsigning off`, reativar Secure Boot).

### Etapa C — Ferramentas e qualidade
- [ ] **F8-T12 — `tools/audio-probe`:** captura do endpoint por WASAPI por N s; calcula **frequência do tom (FFT)**, **SNR**, **descontinuidades** (saltos de amostra/estalos), **duração e contagem de amostras versus relógio**; JSON + código de saída.
- [ ] **F8-T13 — `mock-sender --av-sync`** (flash + bipe com o mesmo `ptsUs`) e **`tools/avsync-probe`** (mede o atraso entre o flash na câmera virtual e o bipe no microfone virtual).
- [ ] **F8-T14 — Teste de deriva e *soak* de áudio** (`scripts/soak.ps1 -Audio`).
- [ ] **F8-T15 — Testes e relatório** (§8).

## 7. Armadilhas conhecidas
- **Dois escritores** no driver = chiado. Abertura exclusiva.
- Relógios do celular e do PC divergem (centenas de ppm): sem correção de deriva, o áudio acumula atraso ou estala em minutos.
- Alguns apps abrem o microfone em **modo exclusivo** ou pedem 44,1/16 kHz: o Windows converte, mas teste ao menos um app assim.
- O Android 12+ tem a chave de privacidade de microfone: a captura devolve **silêncio** sem erro.
- Iniciar a captura de microfone a partir do segundo plano é bloqueado em versões recentes do Android.
- Driver com *bug* causa tela azul: sempre VM/PC de teste, com *Driver Verifier* ligado (§8.6).

## 8. Plano de testes

### 8.1 Unitários — áudio
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-8.01 | Jitter buffer: chegada regular, com jitter ±30 ms, com rajada, com lacuna | Saída contínua; ocupação dentro de [40, 120] ms; contadores de *underrun*/*overrun* corretos | ✅ |
| UT-8.02 | Correção de deriva: relógio de origem +300 ppm / −300 ppm por 30 min simulados | Ocupação estável (sem crescer/esvaziar); contagem de amostras igual à esperada ± 0,1% | ✅ |
| UT-8.03 | Reamostrador: ganho unitário e continuidade de fase (tom de 440 Hz) | Sem descontinuidades; distorção abaixo do limite | ✅ |
| UT-8.04 | Ganho 0%, 100%, 200% e limitador | Sem *clipping* abrupto; 0% = silêncio | ✅ |
| UT-8.05 | Medidor RMS/pico: senoide de amplitude conhecida | Valores em dBFS corretos | ✅ |
| UT-8.06 | Atraso de áudio: 0, 50, 500 ms | Deslocamento exato em amostras | ✅ |
| UT-8.07 | Estimador de offset de relógio (A/V) | Converge em ≤ 2 s; robusto a jitter | ✅ |
| UT-8.08 | Fila de áudio do celular: acima de 200 ms descarta os mais antigos | Conforme regra (relógio virtual) | ✅ |
| UT-8.09 | Ticket e socket de áudio (reabertura após queda) | Conforme protocolo §3 | ✅ |
| UT-8.10 | Detecção de "microfone bloqueado" (2 s de zeros) | Aviso emitido uma vez | ✅ |
| UT-8.11 | RTL: aba Áudio (switch, medidor, ganho, atraso, saída interina visível só sem driver) | Conforme `ui/desktop.md` | ✅ |

### 8.2 Unitários — biblioteca do driver (modo usuário)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-8.20 | Buffer circular: escrita/leitura com *wrap*, cheio, vazio | Dados íntegros; sem *overflow* | ✅ |
| UT-8.21 | *Underrun* | Entrega silêncio e conta | ✅ |
| UT-8.22 | Escritor único: segunda abertura | Recusada | ✅ |
| UT-8.23 | Validação de IOCTL: tamanhos 0, 1, ímpar, gigante, alinhamento | Rejeitados sem acesso fora do limite | ✅ |
| UT-8.24 | Conversão de formato (mono 48 k → o que o endpoint anuncia) | Valores corretos | ✅ |

### 8.3 Integração (com `audio-probe`)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-8.01 | `mock --audio --tone 440` → desktop → microfone virtual → `audio-probe` 30 s | Pico em 440 ± 2 Hz; SNR ≥ 40 dB; **0 descontinuidades**; amostras = 48 000 × 30 ± 0,1% | 🔧 |
| IT-8.02 | `mock --av-sync` → `avsync-probe` | Diferença A/V ≤ 40 ms (delay = 0) | 🔧 |
| IT-8.03 | `mock --slow` **só no vídeo** (socket principal) | Áudio continua **sem falhas** | 🔧 |
| IT-8.04 | Derrubar o socket de áudio e reabrir | Silêncio durante a queda e retomada em ≤ 2 s; sem lixo | 🔧 |
| IT-8.05 | Mock com jitter de 50 ms no áudio | Jitter buffer absorve; sem estalos | 🔧 |
| IT-8.06 | Saída interina com dispositivo virtual (se existir) | Mesmo resultado do IT-8.01 | 🔧 |

### 8.4 Aceitação
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-8.01 | Falar no celular com o desktop tocando na saída interina | Áudio audível, latência baixa e estável | ✋ |
| AT-8.02 | Ligar/desligar o microfone pelo PC e pelo celular | Estado converge; nenhum lado toca chiado | ✋ |
| AT-8.03 | Vídeo com cronômetro + palmas | Atraso A/V ajustável até < 40 ms de diferença percebida | ✋ |
| AT-8.04 | Congelar a rede do vídeo (`--slow`) | **Áudio continua fluido** | ✋ |
| AT-8.05 | Microfone virtual em Configurações → Som | Listado; medidor reage | ✋ |
| AT-8.06 | Gravar no Audacity por 5 min | Sem estalos; sem deriva | ✋ |
| AT-8.07 | Discord, OBS e Zoom com o microfone virtual | Funcionam | ✋ |
| AT-8.08 | Fechar o desktop durante uma chamada | Microfone fica mudo (silêncio), sem travar o app | ✋ |
| AT-8.09 | Dois escritores tentando abrir o driver | O segundo é recusado | ✋ |
| AT-8.10 | 30 min de áudio contínuo | Sem acúmulo de latência nem *underruns* audíveis | ✋ |
| AT-8.11 | `docs/spikes/S4.md` | Lista Secure Boot, HVCI, modo de teste e anti-cheat com o resultado nos seus jogos | ✋ |
| AT-8.12 | Um app em **modo exclusivo** e um que peça 16 kHz/44,1 kHz | Funcionam (conversão do Windows) | ✋ |
| AT-8.13 | Chave de privacidade do microfone do Android ligada | Aviso claro no celular e no PC | ✋ |
| AT-8.14 | Chamada de vídeo real (Meet) com webcam + microfone virtuais | Imagem e voz sincronizadas | ✋ |
| AT-8.15 | Procedimento de **instalação e reversão** em VM limpa | Segue o README; tudo removido ao final | ✋ |

### 8.5 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-8.01 | Rede de áudio parada por 2 s e depois liberada | Silêncio e retomada; **sem rajada** de áudio atrasado (descarta o excesso) | 🔧 |
| FT-8.02 | Permissão de microfone revogada com o app aberto | Para o áudio; aviso; vídeo continua | ✋ |
| FT-8.03 | Dispositivo de saída interina removido (USB/Bluetooth) | Troca para o padrão ou pausa com aviso; sem *crash* | ✋ |
| FT-8.04 | Mudar o dispositivo padrão do Windows durante o uso | Comportamento correto/documentado | ✋ |
| FT-8.05 | Suspender/retomar o Windows | Áudio volta em ≤ 5 s | ✋ |
| FT-8.06 | Desinstalar o driver com um app usando o microfone | Sem tela azul; app recebe erro de dispositivo | ✋ |
| FT-8.07 | Celular mudando de rede (queda do Wi-Fi) | Áudio e vídeo reconectam juntos | ✋ |
| FT-8.08 | Bloqueio de tela / segundo plano no celular | Áudio continua se "transmitir com a tela apagada" estiver ligado | ✋ |
| FT-8.09 | Chamada telefônica entrando | Áudio pausa e retoma de forma limpa | ✋ |
| FT-8.10 | 100 inícios/paradas de captura em Discord | Sem vazamento; sem tela azul | ✋ |

### 8.6 Segurança e estabilidade do driver (só VM/PC de teste)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-8.01 | **Driver Verifier** ligado no driver por todo o conjunto de testes | Sem violações | ✋ |
| ST-8.02 | Análise estática do driver (ferramentas do WDK/CodeQL, **confirmar**) | Sem achados críticos | 🔧 |
| ST-8.03 | Fuzz de IOCTL (tamanhos e conteúdos aleatórios) por 30 min | Sem tela azul nem vazamento | 🔧 |
| ST-8.04 | ACL da interface do dispositivo | Apenas contas esperadas escrevem | ✋ |
| ST-8.05 | Teste de prontidão do HLK para HVCI (ou verificação equivalente) | Compatível | ✋ |
| ST-8.06 | Conteúdo do áudio nos logs | **Nunca** registrado | ✋ |
| ST-8.07 | Indicador de microfone ativo no celular enquanto captura | Aparece (comportamento do sistema); some ao mutar | ✋ |

### 8.7 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-8.01 | Latência de áudio (celular → app no PC), Wi-Fi | ≤ 150 ms (medir com palmas + gravação loopback) | ✋ |
| PT-8.02 | CPU do pipeline de áudio no PC | ≤ 3% de um núcleo | 🔧 |
| PT-8.03 | Erro de sincronia A/V após calibração | ≤ 40 ms | ✋ |
| PT-8.04 | *Underruns* em 30 min | 0 audíveis (contador ≤ 1) | 🔧 |
| PT-8.05 | Deriva em 30 min | Ocupação do buffer estável (± 20 ms) | 🔧 |

### 8.8 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-8.01 | **2 h** de áudio contínuo com `audio-probe`/`soak.ps1` | Sem estalos; deriva dentro do limite; memória estável | 🔧 |
| LT-8.02 | 1 h de webcam + microfone em chamada real | Estável; A/V sincronizados | ✋ |
| LT-8.03 | 50 ciclos de instalar/desinstalar o driver (VM) | Sem resíduos; sem tela azul | ✋ |

### 8.9 Regressão
**RS-S** + AT-4 (câmera) + AT-6 (controles) + o **RS-F** antes de `v0.4.0`.

## 9. Entrega do marco M4 (`v0.4.0`)
- [ ] Testes obrigatórios aprovados; instruções de instalação **e de reversão** documentadas; impacto no sistema (Secure Boot/HVCI/anti-cheat) registrado.
- [ ] `docs/tests/fase-8.md` aprovado; tag `v0.4.0`.

## 10. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| Nenhum driver existente serve e o próprio é inviável | Decisão (4) do S4: saída interina + cabo virtual do usuário (fora do repositório) |
| HVCI bloqueia o driver de teste | Desligar HVCI na VM de teste; avaliar driver de terceiros assinado |
| Anti-cheat do usuário incompatível com modo de teste | Entrada de boot separada ou driver assinado de terceiros |
