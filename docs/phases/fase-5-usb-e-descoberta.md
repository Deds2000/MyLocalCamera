# Fase 5 — USB e descoberta (mDNS)

**Marco:** M2 (`v0.2.0`) · **Tamanho:** M (≈ 7–10 dias) · **Depende de:** Fases 1–4
**Relatório:** `docs/tests/fase-5.md`

## 1. Objetivo e resultado observável
Conectar por **cabo USB** com um clique e **reconectar por Wi-Fi sem escanear o QR de novo**, mesmo quando o IP do PC muda.

**Resultado observável:** plugar o cabo → "Conectar" → vídeo em ≤ 3 s, sem digitar nada; e, em Wi-Fi, mudar o IP do PC e tocar em "Reconectar" no celular → conecta por mDNS em ≤ 4 s.

## 2. Leitura obrigatória
`docs/protocol.md` §4, §5, §6.4, §12; `docs/ui/desktop.md` §2.6.1 e §2.7; `docs/ui/mobile.md` §3.4, §3.8 (sobreposição de reconexão) e §3.9; `docs/testing.md`.

## 3. Escopo
**Dentro:** gerenciador de ADB; `adb reverse`; deep link; janela de aprovação USB; anúncio mDNS no PC; descoberta NSD no celular; cadeia de reconexão; lista de PCs conhecidos; indicadores de saúde (ADB e descoberta).
**Fora:** instalação do APK via ADB (backlog); TLS.

## 4. Entradas e saídas
| Entradas | Saídas |
|---|---|
| Servidor/aprovação (F1), UI (F3), mobile (F2), `fake-adb` | Módulos `adb/` e `discovery/`; `Discovery.kt`; cadeia de reconexão; UI de USB e de PCs conhecidos; `tools/fake-adb` (executável falso para testes); `docs/tests/fase-5.md` |

## 5. Plano de trabalho

### Etapa A — ADB no desktop
- [ ] **F5-T01 — Localizar o `adb`** na ordem: configuração do usuário → `PATH` → `ANDROID_HOME`/`ANDROID_SDK_ROOT` → pasta de dados do app. Se não achar, oferecer **baixar o platform-tools oficial do Google** (com diálogo de consentimento, `ui/desktop.md` §2.8.3), extraindo na pasta de dados do app. Conferir integridade (HTTPS e, se o Google publicar, *hash*; **confirmar**). Não embutir o ADB no repositório. Não encerrar o servidor ADB do usuário ao sair.
- [ ] **F5-T02 — Rastrear aparelhos:** `adb track-devices` (ou *polling* de `adb devices -l` a cada 2 s). Estados: `device`, `unauthorized`, `offline`, ausente; extrair `serial`, `model`, `transport_id` quando existir. Tratar linhas de "daemon starting", fim de linha CRLF e saída vazia.
- [ ] **F5-T03 — Fluxo "Conectar":** (1) abrir a **janela de aprovação USB** (15 s, F1-T07); (2) `adb -s <serial> reverse tcp:<porta> tcp:<porta>`; (3) verificar se o app está instalado (`pm list packages com.localcamera.app`); (4) abrir o app com o deep link:
  `adb -s <serial> shell am start -a android.intent.action.VIEW -d '<URI>' com.localcamera.app`
  **Atenção:** a URI contém `&`; no *shell* do Android, `&` manda o comando para segundo plano e **corta a URI**. Sempre passar a URI **entre aspas simples** dentro do comando remoto (e escapar corretamente no PowerShell/`Command` do Rust). Teste obrigatório: UT-5.03.
- [ ] **F5-T04 — UI:** lista de aparelhos USB no estado vazio e na aba Conexão (estados, mensagens e ações de `ui/desktop.md` §2.6.1); erro claro se o app não estiver instalado ("Instale o APK do LocalCamera no celular").
- [ ] **F5-T05 — Ciclo de vida:** remover o `reverse` ao desconectar (`adb reverse --remove`); cabo puxado → sessão cai → "Reconectando" → "Aparelho removido"; vários aparelhos plugados (cada um com seu `reverse`; só um ativo por vez).
- [ ] **F5-T11 — "Conectar automaticamente ao plugar"** (opcional do usuário, padrão desligado): ao aparecer um aparelho `device` conhecido, iniciar o fluxo F5-T03.

### Etapa B — mDNS e reconexão
- [ ] **F5-T06 — Anúncio (Rust)** com `mdns-sd` (ou equivalente; **confirmar**): `_localcamera._tcp.local.`, instância = nome do PC (tratar colisão de nome), TXT `v`, `id`, `name`, `port` (**sem segredos**); anunciar nas interfaces reais; **reanunciar** ao mudar rede/porta; encerrar o anúncio ao sair; indicador "Descoberta" no rodapé.
- [ ] **F5-T07 — Firewall e rede:** regra de entrada UDP 5353 no `dev-setup.ps1`; documentar o caso de roteador com **isolamento de clientes**.
- [ ] **F5-T08 — `Discovery.kt` (`NsdManager`):** procurar por `_localcamera._tcp`, **filtrar por `pcId`**, resolver (tratar a limitação de resoluções simultâneas e as diferenças entre versões do Android; **confirmar na documentação**), timeout de 3 s, apenas IPv4 privado; liberar o *listener* ao sair.
- [ ] **F5-T09 — Cadeia de reconexão** (`PrimaryConnection`): (1) mDNS (até 3 s) → (2) últimos `hosts` em paralelo (2 s) → (3) falhou: pedir QR/endereço manual. *Backoff* entre rodadas (1-2-4-8 s). A UI mostra a etapa atual (`mobile.reconnect.*`).
- [ ] **F5-T10 — Tela "PCs conhecidos"** (`ui/mobile.md` §3.9): conectar, esquecer, "visto há…"; atualizar `hosts` do PC com o último IP que funcionou.
- [ ] **F5-T12 — `tools/fake-adb`:** executável falso que lê um roteiro (JSON) e devolve saídas/erros determinísticos (inclui `device`, `unauthorized`, `offline`, vários aparelhos, `no devices/emulators found`, falha de `reverse`, app ausente, travamento) — base dos testes de integração.
- [ ] **F5-T13 — Testes e relatório** (ver §7).

## 6. Armadilhas conhecidas
- **`&` na URI do `am start`** (ver F5-T03).
- Adaptadores virtuais (Hyper-V, WSL, VPN) geram IPs inúteis no QR/mDNS: filtrar e permitir escolha.
- O Android pode exigir **driver USB do fabricante** no Windows para o `adb` enxergar o aparelho; a UI deve dar a dica.
- `adb` pode estar em uso por outra ferramenta (Android Studio) e reiniciar o servidor: não matar o servidor alheio; reconectar com *backoff*.
- mDNS usa multicast: não atravessa sub-redes; PC no cabo e celular no Wi-Fi do mesmo roteador geralmente funciona, mas depende do roteador.
- O NSD do Android pode devolver resultados atrasados ou duplicados: deduplicar por `pcId`.

## 7. Plano de testes

### 7.1 Unitários
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-5.01 | Parser de `adb devices -l` (amostras reais: `device`, `unauthorized`, `offline`, `no permissions`, vários, com `model:`/`transport_id:`, CRLF, mensagens de daemon, vazio) | Lista correta de aparelhos e estados | ✅ |
| UT-5.02 | Localização do `adb`: ordem de busca (config → PATH → ANDROID_HOME → pasta do app) com sistema de arquivos falso | Ordem respeitada | ✅ |
| UT-5.03 | **Montagem do comando `am start`:** URI com `&`, espaços e acentos no `name` | A URI chega **inteira** ao aparelho (teste com `fake-adb` que ecoa os argumentos) | ✅ |
| UT-5.04 | Fluxo "Conectar": ordem janela USB → `reverse` → verificação do app → `am start`; falha em cada passo | Para no passo certo e mapeia o erro para a mensagem certa | ✅ |
| UT-5.05 | Janela USB (Fase 1) integrada: expira/uso único | Conforme especificação | ✅ |
| UT-5.06 | Codificação/decodificação do TXT do mDNS; ausência de segredos | Campos corretos; nenhum `token`/`key` | ✅ |
| UT-5.07 | Filtro de IPs (privados, não virtuais, IPv4) | Lista esperada para um conjunto de adaptadores simulado | ✅ |
| UT-5.08 | Cadeia de reconexão (relógio virtual): mDNS ok / mDNS falha→IPs ok / tudo falha | Etapas, tempos e mensagens corretos | ✅ |
| UT-5.09 | Ranking e atualização de `hosts` em `PairingStore` | Último IP que funcionou primeiro | ✅ |
| UT-5.10 | Deduplicação de resultados do NSD por `pcId` | Um resultado por PC | ✅ |
| UT-5.11 | RTL: lista de aparelhos USB (cada estado), PCs conhecidos (vazio, 1, vários, esquecer com confirmação) | Conforme `ui/desktop.md`/`ui/mobile.md` | ✅ |

### 7.2 Integração
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-5.01 | `fake-adb` com um aparelho `device`: clicar "Conectar" (comando de teste) | Janela USB, `reverse`, `am start` na ordem certa; mock conecta de `127.0.0.1` e é aprovado sem diálogo | 🔧 |
| IT-5.02 | `fake-adb` `unauthorized` → `device` (autorizou) | UI muda de estado em ≤ 2 s | 🔧 |
| IT-5.03 | Dois aparelhos simultâneos | Cada um com seu `reverse`; conexão só do selecionado | 🔧 |
| IT-5.04 | Anúncio mDNS + cliente de teste na mesma máquina | Descobre `pcId`, IP e porta corretos | 🔧 |
| IT-5.05 | Mudar a porta do servidor | Reanúncio com a nova porta em ≤ 2 s | 🔧 |
| IT-5.06 | `androidTest`: `Discovery` contra um anunciante de teste na rede | Resolve em ≤ 3 s | 🔧 |

### 7.3 Aceitação
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-5.01 | Plugar o cabo (depuração autorizada) e clicar "Conectar" | App abre sozinho; vídeo flui **sem diálogo** | ✋ |
| AT-5.02 | Cabo com depuração **não** autorizada | UI mostra "autorize no celular" | ✋ |
| AT-5.03 | Puxar o cabo durante a transmissão | Sessão cai de forma limpa; "aparelho removido" | ✋ |
| AT-5.04 | Conexão em `127.0.0.1` **fora** da janela de 15 s | Fluxo normal (aprovação/chave) | ✋ |
| AT-5.05 | Mudar o IP do PC e tocar em Reconectar no celular | Reconecta via mDNS sem QR | ✋ |
| AT-5.06 | Bloquear mDNS (desligar a regra) | Cai para os últimos IPs; se mudaram, pede QR | ✋ |
| AT-5.07 | Dois PCs na mesma rede com o app | Conecta ao `pcId` correto | ✋ |
| AT-5.08 | Esquecer o PC no celular | Próxima conexão exige novo pareamento e aprovação | ✋ |
| AT-5.09 | `adb` ausente | UI oferece baixar o platform-tools; baixa e funciona | ✋ |
| AT-5.10 | Rodapé | "Descoberta" e "ADB" corretos | ✋ |
| AT-5.11 | Cabo USB 2.0 e 3.0: transmitir 1080p30 por 5 min | Sem quedas; latência menor que no Wi-Fi (registrar) | ✋ |
| AT-5.12 | Roteador com **isolamento de clientes** ligado | Descoberta/IPs falham com mensagem clara; USB continua funcionando | ✋ |
| AT-5.13 | PC com VPN e adaptadores virtuais | QR/mDNS usam o IP correto; seletor de interface permite corrigir | ✋ |
| AT-5.14 | "Conectar automaticamente ao plugar" ligado | Plugar o cabo inicia a conexão sozinha | ✋ |
| AT-5.15 | App do celular **não instalado** | Mensagem "Instale o APK" no desktop | ✋ |

### 7.4 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-5.01 | Servidor ADB morre durante a sessão | Sessão cai; `adb` reiniciado de forma limpa; UI se recupera | 🔧 |
| FT-5.02 | Cabo "intermitente" (plugar/desplugar 30× em 2 min) | Sem travar nem vazar `reverse`; UI coerente | ✋ |
| FT-5.03 | Depuração USB revogada com o cabo conectado | Estado vai a `unauthorized`; sessão cai | ✋ |
| FT-5.04 | `adb` ocupado por outra ferramenta (Android Studio) | Sem briga pelo servidor; mensagem se necessário | ✋ |
| FT-5.05 | Rede troca de IP **durante** a transmissão por Wi-Fi | Reconexão automática pela cadeia | ✋ |
| FT-5.06 | PC no cabo e celular no Wi-Fi (mesmo roteador) | mDNS funciona ou cai para IPs; comportamento documentado | ✋ |
| FT-5.07 | Dois anunciantes com o mesmo nome de PC | Nomes únicos; conexão correta por `pcId` | 🔧 |
| FT-5.08 | `reverse` falha (porta já em uso no aparelho) | Mensagem clara e tentativa de limpeza | 🔧 |
| FT-5.09 | Aparelho dorme/bloqueia durante a conexão USB | O app é aberto assim mesmo ou a UI orienta a desbloquear | ✋ |
| FT-5.10 | Download do platform-tools falha no meio | Retomável/sem arquivo corrompido; mensagem | 🔧 |

### 7.5 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-5.01 | Tempo clique "Conectar" → vídeo no preview (USB) | ≤ 3 s (registrar mediana de 10 tentativas) | ✋ |
| PT-5.02 | Tempo "Reconectar" → vídeo via mDNS | ≤ 4 s | ✋ |
| PT-5.03 | Taxa de sucesso do mDNS em 20 tentativas na rede doméstica | ≥ 95% | ✋ |
| PT-5.04 | CPU do `track-devices` e do anúncio em repouso | ≈ 0% | 🔧 |
| PT-5.05 | Latência vidro a vidro por USB | ≤ 150 ms | ✋ |

### 7.6 Segurança
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-5.01 | TXT do mDNS capturado (ex.: Wireshark/`dns-sd`) | Sem token, chave ou ticket | ✋ |
| ST-5.02 | Deep link com `token` nos logs do desktop e no `logcat` | Não aparece completo | ✋ |
| ST-5.03 | Janela USB: conexão de `127.0.0.1` por **outro processo** fora dos 15 s | Segue o fluxo normal (aprovação) | ✅ |
| ST-5.04 | Aprovação automática só uma vez | Segundo `HELLO` na mesma janela não é auto-aprovado | ✅ |
| ST-5.05 | Verificação do download do platform-tools (origem HTTPS oficial, destino correto) | Conforme o consentimento exibido | ✋ |
| ST-5.06 | Injeção em argumentos de `adb` (serial/URI com aspas e `;`) | Nada é executado além do pretendido | ✅ |

### 7.7 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-5.01 | 30 plugs/unplugs do cabo | Sem vazamento de processos `adb` nem de `reverse` | ✋ |
| LT-5.02 | 60 min por USB com o celular real | Estável; fps e latência estáveis | ✋ |
| LT-5.03 | 24 h com o desktop aberto e anunciando mDNS (sem celular) | CPU ≈ 0%; memória estável | 🔧 |

### 7.8 Regressão
**RS-S** + AT-2.xx (pareamento, reconexão) + AT-4.14.

## 8. Checklist de saída
- [ ] Testes obrigatórios aprovados; metas de PT registradas.
- [ ] README atualizado (USB, driver do fabricante, mDNS e roteadores).
- [ ] `docs/tests/fase-5.md` preenchido.

## 9. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| NSD do Android instável em alguns aparelhos | Cadeia com IPs conhecidos e QR; opcionalmente biblioteca mDNS própria (ADR) |
| Roteadores bloqueando multicast | Documentar; USB e QR continuam |
| Driver USB do fabricante ausente no Windows | Mensagem de ajuda com instrução genérica |
