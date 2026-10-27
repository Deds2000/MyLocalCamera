# Fase 4 — Câmera virtual (fecha o marco M1)

**Marco:** M1 → tag `v0.1.0` · **Tamanho:** G (≈ 12–18 dias) · **Depende de:** Fase 0 (S1, S2), Fase 3
**Relatório:** `docs/tests/fase-4.md`

## 1. Objetivo e resultado observável
O dispositivo **"LocalCamera"** aparece e funciona como webcam no Windows 11 — **inclusive como entrada do NVIDIA Broadcast** — mostrando o vídeo do celular, e um quadro de espera ("Aguardando celular…") quando não há celular.

**Resultado observável:** a matriz de apps (AT-4.01–AT-4.13) passa e o `vcam-probe` valida automaticamente formato, taxa, orientação e ordem dos quadros.

## 2. Leitura obrigatória
`docs/spikes/S1.md` e `S2.md` (**os vereditos mandam**), `docs/composition.md`, `docs/testing.md` §4.2–4.3, documentação oficial de `MFCreateVirtualCamera` e o exemplo `Windows-Camera/Samples/VirtualCamera` (confirmar caminho).

## 3. Escopo
**Dentro:** DLL C++, registro pelo app, canal de frames, placeholder, indicador "em uso", `vcam-probe`, script de instalação de desenvolvimento, plano B DirectShow (se o S1 mandar).
**Fora:** duas câmeras e PiP (7), instalador final (9).

## 4. Entradas e saídas
| Entradas | Saídas |
|---|---|
| Veredito S1/S2, decodificador e composição da Fase 3 | `virtualcam/` (DLL); módulo `vcam/` no desktop; `tools/vcam-probe`; `scripts/dev-setup.ps1`; README com matriz de apps; `docs/tests/fase-4.md`; tag `v0.1.0` |

## 5. Plano de trabalho

### Etapa A — Projeto da DLL
Projeto (confirmar cada item na documentação):
1. **A DLL é carregada pelo Frame Server**, com outra conta e outra sessão. Portanto: instalar em caminho legível pelo serviço (`C:\Program Files\LocalCamera\`), **nunca** na pasta do usuário; registrar o CLSID (`regsvr32`, administrador, uma vez); o app registra a câmera com `MFCreateVirtualCamera` (tempo de vida/acesso conforme o S1) e chama `Start`.
2. **Canal de frames** conforme o S2 (esperado: *named pipe* com DACL para o Frame Server, rejeitando clientes remotos).
3. **Formato:** NV12 1920×1080 a 30 fps como tipo principal (plano Y de 1920×1080 seguido do plano UV intercalado 1920×540; *stride* correto); atributos de mídia (tamanho, taxa, subtipo, progressivo, proporção de pixel 1:1) conforme a documentação.
4. **Temporização:** a DLL gera um **ritmo de 30 fps** próprio. A cada tick: se há quadro novo, entrega; senão, **repete o último** (até 1 s); depois disso, **placeholder**. Carimbos de tempo monotônicos.
5. **Placeholder:** bitmap embutido "Aguardando celular…" com logotipo (NV12 pré-renderizado).
6. **Concorrência e ciclo de vida:** estruturas protegidas; contagem de referências correta; `DllCanUnloadNow`; tolerar o Frame Server reiniciar e o app reiniciar.

- [ ] **F4-T01 — `virtualcam/`:** projeto CMake + vcpkg (se usado); DLL COM baseada no exemplo da Microsoft (`IMFMediaSourceEx`, stream `IMFMediaStream2`, atributos de câmera); `DllRegisterServer/DllUnregisterServer`.
- [ ] **F4-T02 — Cliente do canal na DLL:** conectar, ler cabeçalho `{w,h,format,seq,ptsUs}` + frame, desconexão, reconexão; **sem bloquear** o tick de 30 fps.
- [ ] **F4-T03 — Placeholder e *timeouts*** (> 1 s sem quadros).
- [ ] **F4-T08 — Plano B DirectShow**, **se** o S1 mandar (filtro alimentado pelo mesmo canal; ambos registrados quando úteis).

### Etapa B — Lado do app (Rust)
- [ ] **F4-T04 — `vcam/`:** registro/remoção da câmera; **servidor do canal** com DACL conforme o S2; **detecção de cliente** (só enviar quando há cliente; expor "em uso" e a contagem); envio do canvas NV12 composto a 30 fps (Fase 3) com **contrapressão**: escrita não bloqueante, pipe cheio = descartar o quadro e contar.
- [ ] **F4-T05 — Indicador "Câmera virtual"** no rodapé: não registrada / registrada / **em uso** / erro (com causa).
- [ ] **F4-T07 — Assistente de primeiro uso:** detecta "não registrada" e oferece **Corrigir** (UAC uma vez).
- [ ] **F4-T11 — Métricas internas:** latência recepção→entregue ao pipe (p50/p95), quadros descartados no pipe, fps entregue.

### Etapa C — Instalação de desenvolvimento
- [ ] **F4-T06 — `scripts/dev-setup.ps1`:** copia a DLL para `Program Files`; **ajusta as permissões** para que usuários comuns **não** possam alterá-la (evita sequestro de DLL por um serviço); `regsvr32`; regra de firewall de entrada (TCP na porta configurada); imprime diagnóstico; `-Uninstall` desfaz tudo e **remove a câmera**.
- [ ] **F4-T10 — README:** requisitos, passos, matriz de apps testados (com versões).

### Etapa D — Ferramenta de verificação
- [ ] **F4-T12 — `tools/vcam-probe`:** abre a câmera **como um app qualquer** (leitor de fontes do Media Foundation), lê N quadros e valida: resolução e formato; **fps** e **jitter** (desvio-padrão do intervalo); **contador de quadros do `lc-pattern`** (perdidos/repetidos/fora de ordem); **cores por quadrante** (orientação, espelhamento, NV12 correto); detecção de **placeholder**; saída em JSON e código de saída ≠ 0 se algum critério falhar.
- [ ] **F4-T09 — Testes de longa duração e de abrir/fechar 50×** (ver §7).

## 6. Armadilhas conhecidas
- Sessão remota (RDP) e *fast user switching* podem comportar-se diferente com câmeras virtuais; **documentar** o resultado.
- Apps empacotados (UWP/Store) podem exigir o registro como câmera Media Foundation (não só DirectShow) — por isso o S1.
- Câmeras virtuais costumam falhar quando a DLL demora a responder: o tick **nunca** pode esperar o pipe.
- Se o Windows entrar em suspensão e voltar, o pipe e o Frame Server podem ter reiniciado: tratar reconexão.
- O usuário pode ter outra câmera virtual (OBS etc.); os nomes não devem colidir.

## 7. Plano de testes

### 7.1 Unitários (C++ e Rust)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-4.01 | Cliente do pipe (C++): cabeçalho válido, truncado, tamanho absurdo, `format` desconhecido | Aceita/ignora/fecha conforme regras; sem *crash* | ✅ |
| UT-4.02 | Gerador de placeholder: tamanho do buffer NV12 = 1920×1080×3/2; planos corretos | Bytes e layout corretos | ✅ |
| UT-4.03 | Agendador de 30 fps: sem quadro novo repete o último até 1 s, depois placeholder; com quadro novo entrega-o | Sequência correta (relógio virtual) | ✅ |
| UT-4.04 | Carimbos de tempo monotônicos mesmo com relógio do app andando para trás | Nunca decresce | ✅ |
| UT-4.05 | Conversão/escrita NV12 do app: *stride* e alinhamento | Layout idêntico ao esperado | ✅ |
| UT-4.06 | Servidor do pipe (Rust): conecta/desconecta cliente → estado "em uso" correto | Contagem correta | 🔧 |
| UT-4.07 | Contrapressão: cliente lento | Descarta quadros e conta; sem crescer memória | 🔧 |
| UT-4.08 | Construção da DACL do pipe | Contém só as contas esperadas; rejeita clientes remotos | 🔧 |

### 7.2 Integração (com `vcam-probe`)
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-4.01 | mock (`pattern1080.h264`) → desktop → câmera virtual → `vcam-probe` por 60 s | 1920×1080 NV12 @30 ± 0,3; **0 quadros perdidos/repetidos/fora de ordem**; quadrantes nas cores certas | 🔧 |
| IT-4.02 | Mesmo teste com `--rotation-hint 90` e entrada 4:3 | Orientação/letterbox conforme `composition.md` | 🔧 |
| IT-4.03 | Sem celular, `vcam-probe` abre a câmera | Detecta o **placeholder** | 🔧 |
| IT-4.04 | `vcam-probe` aberto → mock conecta → desconecta | placeholder → vídeo → placeholder, **sem fechar a câmera** | 🔧 |
| IT-4.05 | Dois `vcam-probe` simultâneos | Ambos recebem quadros corretos (ou comportamento documentado) | 🔧 |
| IT-4.06 | `vcam-probe` com *seed* de `--loss 5` no mock | Câmera continua a 30 fps (repete o último); sem travar | 🔧 |
| IT-4.07 | Latência recepção→câmera | p95 ≤ 30 ms | 🔧 |

### 7.3 Aceitação (matriz de apps)
| ID | App / cenário | Resultado esperado | Auto |
|---|---|---|---|
| AT-4.01 | **NVIDIA Broadcast** (entrada = LocalCamera) | Listada, com imagem; efeitos aplicáveis | ✋ |
| AT-4.02 | Câmera do Windows / Configurações → Câmeras | Listada, com imagem | ✋ |
| AT-4.03 | Chrome (Google Meet) | Listada, imagem fluida | ✋ |
| AT-4.04 | OBS (Dispositivo de captura de vídeo) | Listada, 1080p30 | ✋ |
| AT-4.05 | Zoom e Teams | Listada, imagem fluida | ✋ |
| AT-4.06 | Discord | Listada, imagem fluida | ✋ |
| AT-4.07 | Abrir o app de vídeo **antes** de conectar o celular | "Aguardando celular…"; ao conectar, a imagem entra sem reabrir a câmera | ✋ |
| AT-4.08 | Desconectar o celular com a câmera aberta | Volta ao placeholder; app consumidor não trava | ✋ |
| AT-4.09 | Fechar o desktop com a câmera aberta | Congela/some de forma limpa; apps não travam | ✋ |
| AT-4.10 | Dois apps usando a câmera ao mesmo tempo | Comportamento documentado | ✋ |
| AT-4.11 | Abrir/fechar a câmera 50 vezes | Sem vazamento de handles nem queda de fps | ✋ |
| AT-4.12 | Rodar o app **sem** administrador após o registro | Funciona | ✋ |
| AT-4.13 | Rodapé | Indica "Em uso" enquanto algum app usa a câmera | ✋ |
| AT-4.14 | Cadeia completa com celular real: QR → aprovação → Meet mostrando a imagem | Imagem do celular no Meet com latência aceitável | ✋ |
| AT-4.15 | NVIDIA Broadcast → saída "Camera - NVIDIA Broadcast" usada no Zoom | Efeito (ex.: desfoque de fundo) aplicado sobre o vídeo do celular | ✋ |

### 7.4 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-4.01 | Encerrar o processo do desktop à força com a câmera aberta | Consumidor continua (placeholder/último quadro); sem travar | 🔧 |
| FT-4.02 | Reiniciar o serviço Frame Server com o desktop ativo | Câmera é recriada/volta; app reconecta o canal | ✋ |
| FT-4.03 | Remover a DLL/desregistrar com a câmera em uso | Erro tratado; rodapé em `danger`; sem *crash* do desktop | ✋ |
| FT-4.04 | Registrar duas vezes / registrar com a DLL ausente | Mensagem clara; sem duplicar dispositivos | 🔧 |
| FT-4.05 | Suspender e retomar o Windows durante o uso | Câmera volta sozinha em ≤ 5 s | ✋ |
| FT-4.06 | Encerrar sessão/*fast user switching* e voltar | Sem câmeras fantasmas; app funciona | ✋ |
| FT-4.07 | Pipe cheio (consumidor da DLL bloqueado artificialmente) | App descarta quadros; sem travar a UI | 🔧 |
| FT-4.08 | Quedas repetidas do celular (`--reconnect 30`) com a câmera aberta | A câmera nunca fecha; sem vazamento | 🔧 |
| FT-4.09 | Sessão remota (RDP) | Comportamento documentado | ✋ |
| FT-4.10 | Outra câmera virtual (OBS) instalada ao mesmo tempo | Sem conflito de nomes/CLSID | ✋ |

### 7.5 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-4.01 | Variação do intervalo entre quadros (probe, 5 min) | Desvio-padrão ≤ 3 ms; sem intervalos > 66 ms | 🔧 |
| PT-4.02 | CPU do envio ao pipe com 1080p30 | ≤ 5% de um núcleo (registrar) | 🔧 |
| PT-4.03 | Latência recepção→câmera p95 | ≤ 30 ms | 🔧 |
| PT-4.04 | Latência vidro a vidro (celular real → Meet) | ≤ 250 ms Wi-Fi / ≤ 150 ms USB (USB na Fase 5) | ✋ |
| PT-4.05 | Quadros descartados no pipe em regime | 0 (steady state) | 🔧 |

### 7.6 Segurança
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-4.01 | Cliente de **outro usuário** do Windows tenta abrir o pipe | Negado | ✋ |
| ST-4.02 | Cliente **remoto** (outra máquina) tenta abrir o pipe | Negado | ✋ |
| ST-4.03 | Usuário comum tenta substituir a DLL em `Program Files` | Negado (ACL correta) | ✋ |
| ST-4.04 | Pipe com mensagens malformadas/gigantes | A DLL e o app ignoram/fecham sem *crash* | ✅ |
| ST-4.05 | A DLL não escreve nem lê arquivos do usuário | Verificado por inspeção/Process Monitor | ✋ |
| ST-4.06 | `dev-setup.ps1 -Uninstall` | Remove DLL, chaves de registro, regra de firewall e a câmera | ✋ |

### 7.7 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-4.01 | **60 min** com `vcam-probe` + Meet aberto | Sem queda de fps; memória/handles estáveis; **0** quadros fora de ordem | 🔧 |
| LT-4.02 | 100 conexões/desconexões do celular (mock) com a câmera aberta | Sem vazamento; câmera sempre ativa | 🔧 |
| LT-4.03 | Abrir/fechar a câmera 200× pelo probe | Handles estáveis | 🔧 |

### 7.8 Regressão
**RS-S** + AT-1/2/3 (smoke). Antes da tag `v0.1.0`, rodar **RS-F** (exceto itens das fases futuras).

## 8. Entrega do marco M1 (`v0.1.0`)
- [ ] Todos os testes obrigatórios das Fases 0–4 aprovados; PT dentro das metas ou desvio por ADR.
- [ ] README com requisitos, instalação de desenvolvimento e matriz de apps/versões.
- [ ] `docs/tests/fase-4.md` aprovado; tag `v0.1.0`.

## 9. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| NVIDIA Broadcast não aceita a câmera Media Foundation | Plano B DirectShow (S1); manter os dois |
| Frame Server não aceita a DLL sem assinatura | Assinar com certificado de teste (registrar no ADR) |
| Pipe com permissões insuficientes sem administrador | Memória compartilhada (S2) ou serviço auxiliar com privilégio mínimo (ADR) |
