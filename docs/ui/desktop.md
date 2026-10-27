# Desktop — Especificação de UI (§2 do `ui-spec`)

Pré-requisito: `docs/ui-spec.md` (tokens, glass, componentes, regras de desempenho). Textos em `docs/ui/copy.md`.

---

## 2.1 Janela

| Item | Valor |
|---|---|
| Tamanho inicial | 1280 × 800 (lembrar tamanho/posição entre execuções) |
| Mínimo | 960 × 640 (nada abaixo disso) |
| Decoração | Sem decoração nativa (`decorations: false`); titlebar própria. **Confirmar** na documentação do Tauri 2 o comportamento de cantos arredondados, sombra e *snap layouts* do Windows 11 em janelas sem decoração |
| Instância | Única (`single-instance`): abrir de novo foca a janela existente |
| Fechar (X) | Padrão: encerra o app. Opção em Ajustes: "Minimizar para a bandeja" (desligada por padrão) |
| Tema | Só escuro |
| Layout | Grade de 2 colunas: **preview flexível** + **painel lateral fixo**; titlebar (40 px) e rodapé (32 px) |

**Breakpoints de largura da janela**

| Largura | Painel lateral | Observação |
|---|---|---|
| ≥ 1280 | 380 px | Padrão |
| 1100–1279 | 340 px | |
| 960–1099 | 300 px | Rótulos das abas viram só ícones (com tooltip) |

O preview ocupa todo o espaço restante, mantendo **16:9** centralizado com letterbox em `bg-raised`; margem de 24 px (16 px abaixo de 1100).

## 2.2 Wireframe — estado transmitindo (1280 × 800)

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ◉ LocalCamera     ( ● Pixel 8 · Wi-Fi · 12 ms )                       🌐 PT ▾    ─   ▢   ✕   │ 40
├───────────────────────────────────────────────────────────────┬──────────────────────────────┤
│                                                               │  [🔌 Conexão][📷 Câmera]      │
│   ╭──────────────────────────────────────────────────────╮    │  [🎤 Áudio ][⚙ Ajustes]       │
│   │ (●AO VIVO) [1080p·30] [8,0 Mbps] [12 ms] [0 perdidos]│    │ ──────────────────────────── │
│   │                                                      │    │  ┌ DeviceCard ─────────────┐ │
│   │                                                      │    │  │ 📱 Pixel 8       Wi-Fi  │ │
│   │                    PREVIEW  16:9                     │    │  │ Android 15 · 12 ms      │ │
│   │                                                      │    │  │ [ Desconectar ]         │ │
│   │                                                      │    │  └─────────────────────────┘ │
│   │            ╭────────────────────────────╮            │    │  CÂMERA                      │
│   │            │ ⇋   ⟳   ⟲   ▣   ⛶   🎤     │            │    │  (Traseira|Frontal|Dupla)    │
│   │            ╰────────────────────────────╯            │    │  ...                         │
│   ╰──────────────────────────────────────────────────────╯    │                              │
├───────────────────────────────────────────────────────────────┴──────────────────────────────┤
│ ● Servidor :48650   ● Descoberta   ● Câmera virtual · em uso   ○ Microfone   ● ADB           │ 32
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

## 2.3 Wireframe — estado vazio (sem celular)

```
│   ╭──────────────────────────────────────────────────────╮    │ [🔌 Conexão] ...             │
│   │                                                      │    │ ──────────────────────────── │
│   │          Conecte seu celular                         │    │ ENDEREÇOS                    │
│   │   Escaneie o QR code com o app LocalCamera           │    │ 192.168.0.10  [copiar]       │
│   │                                                      │    │ (▾ outras interfaces)        │
│   │            ┌─────────────────┐                       │    │                              │
│   │            │   ▒▒ QR CODE ▒▒ │   Rede: 192.168.0.10  │    │ APARELHOS USB                │
│   │            │                 │   Porta 48650         │    │ ┌──────────────────────────┐ │
│   │            └─────────────────┘   [↻ Regenerar QR]    │    │ │ 📱 Pixel 8  [Pronto]     │ │
│   │                                                      │    │ │        [ Conectar ]      │ │
│   │      ─────────  ou use o cabo USB  ─────────         │    │ └──────────────────────────┘ │
│   │      Ative a depuração USB no celular e plugue.      │    │ DISPOSITIVOS CONFIÁVEIS      │
│   ╰──────────────────────────────────────────────────────╯    │ (lista / vazio)              │
```

## 2.4 Titlebar (40 px)

| Zona | Conteúdo |
|---|---|
| Esquerda | Ícone (20 px) + "LocalCamera" (logotipo bicolor). Região de arrasto |
| Centro | **Pílula de estado da conexão** (`StatusChip`, §1.9): estado + nome do aparelho + transporte (`Wifi`/`Usb`) + RTT. Em `idle`: "Desconectado". Clique abre a aba Conexão |
| Direita | Menu de idioma (`Languages`; Sistema/Português/English), minimizar, maximizar/restaurar, fechar. O botão fechar fica `danger` no hover |

A região de arrasto cobre tudo, **exceto** os botões e a pílula. Duplo clique na região alterna maximizar.

## 2.5 Preview

### 2.5.1 `PreviewFrame`
- Proporção 16:9 fixa (a saída é sempre 1920×1080; ver `docs/composition.md`), raio `xl`, borda `glass-border`, sombra `shadow-glass`; **sem blur sobre o vídeo**.
- Fundo do letterbox: `bg-raised`.
- Overlays (chips e barra) têm fundo `bg/60` **sólido** com leve transparência — sem `backdrop-filter` sobre o vídeo.

### 2.5.2 Estados do preview

| Estado | O que aparece | Overlays |
|---|---|---|
| `empty` | `EmptyState` com QR (§2.3) | Nenhum |
| `connecting` | Fundo `bg-raised`, spinner e "Conectando…" | Nenhum |
| `pending` | Escudo `ShieldAlert` + "Aguardando sua aprovação" + botão que reabre o diálogo | Nenhum |
| `streaming` | Vídeo | Chips + barra de ferramentas |
| `reconnecting` | **Último quadro congelado** com escurecimento de 50% (opacidade, sem blur) + spinner + "Reconectando… (etapa)" | Chips em `warn` |
| `error` | Ícone `TriangleAlert`, título e mensagem pelo código (`common.errors.*`) + ação (Tentar novamente / Parear de novo) | Nenhum |
| `paused` (janela oculta) | Não visível ao usuário; ao voltar, retoma em < 200 ms sem salto | — |

### 2.5.3 Chips (canto superior esquerdo, 8 px de margem, `gap` 6 px)
1. **AO VIVO** (`StatusChip`, estado `streaming`, ponto pulsando) — vira "RECONECTANDO" em `warn`.
2. **Resolução·fps** (`1080p · 30`) — em modo dual mostra as duas ("720p · 30 | 720p · 30").
3. **Bitrate** (`8,0 Mbps`).
4. **Latência** (`12 ms`, RTT do PING/PONG) — fica `warn` > 80 ms e `danger` > 200 ms.
5. **Perdidos** (quadros descartados na janela de 10 s) — **só aparece se > 0**.
Os valores usam `stat` (mono, tabulares) e atualizam a **≤ 2 Hz**.

### 2.5.4 Barra de ferramentas (pílula centralizada, 16 px acima da base do preview)
Botões de 36 px, ícone 20 px, tooltips com nome + atalho. Some após 3 s sem mouse **somente em tela cheia**; reaparece ao mover o mouse.

| # | Ação | Ícone | Atalho | Aplicado em | Observação |
|---|---|---|---|---|---|
| 1 | Espelhar | `FlipHorizontal2` | `M` | PC | `aria-pressed`; atua na câmera ativa (em dual, na selecionada) |
| 2 | Girar 90° | `RotateCw` | `R` | PC | 0→90→180→270→0 |
| 3 | Alternar câmera | `SwitchCamera` | `C` | Celular | Traseira ↔ frontal; mostra "Aplicando…" ~1 s; some em modo dual |
| 4 | Trocar principal/miniatura | `PictureInPicture2` | `S` | PC | Só no modo dual + PiP |
| 5 | Tela cheia do preview | `Maximize`/`Minimize` | `F` / `F11` | PC | `Esc` sai |
| 6 | Mudo (microfone do celular) | `Mic`/`MicOff` | `Shift+M` | Celular | Estado converge via `STATE` |

### 2.5.5 Modo dual
- **PiP:** o preview mostra exatamente a composição que a câmera virtual entrega (principal em tela cheia + miniatura arredondada). A miniatura pode ser **arrastada** para um dos 4 cantos (encaixa com animação de mola) — equivale à configuração de posição.
- **Duas webcams:** duas molduras lado a lado (50/50), cada uma com legenda ("Traseira", "Frontal") e seus próprios chips compactos; espelho/rotação da barra atuam na moldura **selecionada** (borda `accent`).

### 2.5.6 Tela cheia do preview
Preenche a janela (a janela em si não vira tela cheia do sistema), fundo preto, chips e barra com a regra de auto-ocultar; `Esc`/`F`/`F11` saem.

## 2.6 Painel lateral

Contêiner `surface-1`, raio `lg`, margem de 16 px. Cabeçalho fixo com **abas** (shadcn `Tabs`, estilo "segmented"): **Conexão · Câmera · Áudio · Ajustes**. Conteúdo rola em `ScrollArea`; troca de aba anima (fade + 8 px). A aba selecionada é lembrada.

Convenções: seções com rótulo `section`; cada linha é um `SettingRow` (rótulo à esquerda, controle à direita, descrição `small` abaixo); controles que **reiniciam a captura no celular** mostram o ícone `Smartphone` sutil e, ao aplicar, um spinner por ~1 s.

### 2.6.1 Aba Conexão

**`DeviceCard`** (só quando conectado): ícone, nome do aparelho, modelo, Android, chip do transporte (Wi-Fi/USB), RTT, botão **Desconectar** (secundário; confirmação não é necessária).

**Seção "Parear novo aparelho"** (recolhida quando já há celular conectado):
| Elemento | Detalhe |
|---|---|
| `QRCard` | QR de 220 px com margem clara; legenda "Rede: {ip} · Porta {port}" |
| Seletor de interface | Aparece se houver > 1 IP; `Select` com nome da interface + IP; muda o QR |
| **Regenerar QR** | Botão secundário; abre `AlertDialog`: "Regenerar o código invalida o QR atual. Aparelhos já pareados continuam funcionando." |
| Código de pareamento | Campo somente leitura **mascarado** (••••), botões mostrar/ocultar e copiar (toast "Copiado!") |

**Seção "Aparelhos USB"**: lista de `DeviceRow` (modelo/serial curto, `badge` de estado, ação):
| Estado do ADB | Badge | Ação |
|---|---|---|
| `device` | "Pronto" (`ok`) | **Conectar** (primário) |
| `unauthorized` | "Autorize no celular" (`warn`) | Desabilitado + ajuda "Toque em Permitir na caixa de diálogo do celular" |
| `offline` | "Offline" (muted) | Desabilitado |
| conectando | "Conectando…" | Spinner |
Vazio: texto de ajuda para ativar a depuração USB. Interruptor **"Conectar automaticamente ao plugar"** (padrão desligado). Se o ADB não existe: cartão com **Baixar** (ver Ajustes).

**Seção "Dispositivos confiáveis"**: `DeviceRow` com nome, modelo, "visto por último há …", ação **Remover** (`Trash2`) → `AlertDialog` "Remover *{nome}*? Ele precisará ser pareado e aprovado de novo." Vazio: "Nenhum aparelho pareado."

### 2.6.2 Aba Câmera

| Seção | Controle | Tipo | Valores / padrão | Regras |
|---|---|---|---|---|
| Fonte | **Câmera** | `SegmentedControl` | Traseira · Frontal · Dupla; padrão: Traseira | "Dupla" desabilitada com motivo quando `caps.concurrent=false` ("Seu aparelho não permite usar as duas câmeras ao mesmo tempo") |
| Fonte | **Saída no modo dual** | Cartões de rádio (2) | "Duas webcams" / "Uma webcam (PiP)"; padrão PiP | Só visível em Dupla. Mudar recria as câmeras virtuais: **aviso** "Apps que usam a câmera precisam reabri-la" |
| PiP | **Posição da miniatura** | Grade 2×2 de botões | 4 cantos; padrão inferior direito | Só em PiP |
| PiP | **Tamanho da miniatura** | `Slider` | 15–35%, passo 1, padrão 25% | Só em PiP |
| PiP | **Trocar principal e miniatura** | Botão | — | Só em PiP |
| Qualidade | **Resolução** | `Select` | Opções vindas de `caps` (ex.: 1080p, 720p); padrão 1080p | Em dual, limitada ao máximo simultâneo; mostra aviso se rebaixada ("Reduzido para 720p neste modo") |
| Qualidade | **Taxa de quadros** | `Select` | 24 / 30 / 60 conforme `caps`; padrão 30 | 60 só aparece se suportado |
| Qualidade | **Bitrate automático** | `Switch` | Padrão **ligado** | Desligado habilita o slider |
| Qualidade | **Bitrate** | `Slider` + valor | 2–20 Mbps, passo 0,5; padrão 8 (1080p30) | *Debounce* 300 ms; exibe "{x} Mbps" |
| Imagem | **Espelhar** | `Switch` (por câmera em dual) | Padrão desligado | Aplicado no PC, instantâneo |
| Imagem | **Rotação** | `SegmentedControl` | 0° · 90° · 180° · 270° | Aplicado no PC |
| Imagem | **Ajuste da saída** | `SegmentedControl` | Conter · Preencher; padrão Conter | "Conter" = barras pretas; "Preencher" = corta |

Rodapé da aba: texto `small` "Aplicado no celular" / "Aplicado no PC" explicando os ícones.

### 2.6.3 Aba Áudio
| Controle | Tipo | Valores / padrão | Regras |
|---|---|---|---|
| **Microfone do celular** | `Switch` | Padrão ligado | Reflete o `STATE`; falha de permissão mostra erro inline |
| **Nível** | `LevelMeter` | 24 segmentos, pico retido 1 s | ≤ 20 Hz; cinza quando desligado |
| **Ganho** | `Slider` | 0–200%, passo 5, padrão 100% | Aplicado no PC; aviso `warn` > 150% ("Pode distorcer") |
| **Atraso do áudio** | `Slider` + campo numérico | 0–500 ms, passo 5, padrão 0 | Dica: "Aumente se o som chegar antes da imagem" |
| **Saída (modo interino)** | `Select` | Lista de dispositivos de saída | **Só aparece** quando o microfone virtual não está instalado |
| **Status do microfone virtual** | Linha de saúde | Ver §2.7 | Botão **Corrigir** |

### 2.6.4 Aba Ajustes
| Seção | Controle | Detalhe |
|---|---|---|
| Geral | **Idioma** | `Select`: Sistema / Português / English |
| Geral | **Iniciar com o Windows** | `Switch` (padrão desligado) |
| Geral | **Minimizar para a bandeja ao fechar** | `Switch` (padrão desligado) |
| Rede | **Porta** | Campo numérico 1024–65535; validação inline; mudar mostra banner "Reiniciar o servidor para aplicar" com botão. Se a porta está ocupada: aviso e porta alternativa sugerida |
| Rede | **Regra de firewall** | Status (ok / ausente) + **Corrigir** (UAC) |
| ADB | **Caminho do ADB** | Campo + **Procurar…** + **Baixar platform-tools** (com diálogo de consentimento mostrando a origem oficial do Google e o destino) |
| Câmera virtual | **Registro** | Status + **Reparar** / **Remover** |
| Diagnóstico | **Nível de log** | `Select`: Erro / Aviso / Info / Debug |
| Diagnóstico | **Abrir pasta de logs** / **Exportar diagnóstico** | Botões; exportação **sem** tokens/chaves |
| Sobre | Versões (app, protocolo), licenças de terceiros, link do projeto | |

## 2.7 Rodapé de saúde (`HealthBar`, 32 px)

Cinco `HealthIndicator`s (ponto + rótulo curto). Clique abre um popover com detalhe + ação de correção.

| Indicador | Estado | Ponto | Texto | Ação |
|---|---|---|---|---|
| **Servidor** | Escutando | `ok` | ":{porta}" | — |
| | Porta alternativa | `warn` | ":{porta} (alterada)" | Abrir Ajustes |
| | Falha | `danger` | "Servidor parado" | Tentar novamente |
| **Descoberta** | Ativa | `ok` | "Visível na rede" | — |
| | Desativada/sem multicast | `warn` | "Descoberta indisponível" | Ajuda (QR/IP continuam) |
| **Câmera virtual** | Não registrada | muted | "Câmera não registrada" | **Corrigir** (UAC) |
| | Registrada | `ok` | "Câmera pronta" | — |
| | Em uso | `accent` | "Câmera em uso ({n})" | — |
| | Erro | `danger` | "Erro: {código}" | Reparar / ver log |
| **Microfone virtual** | Não instalado | muted | "Microfone não instalado" | Instalar (guia) |
| | Modo de teste desligado | `warn` | "Ative o modo de teste" | Guia |
| | Bloqueado (integridade de memória) | `warn` | "Bloqueado pelo Windows" | Guia |
| | Ativo | `ok` | "Microfone pronto" | — |
| **ADB** | Encontrado | `ok` | "ADB v{versão}" | — |
| | Ausente | muted | "ADB ausente" | **Baixar** |

(Enquanto a fase do respectivo recurso não foi implementada, o indicador aparece como "Em breve" em `muted`, sem ação.)

## 2.8 Diálogos e fluxos

### 2.8.1 Aprovação de dispositivo (`ApprovalDialog`)
Modal (`AlertDialog`), **não fecha** clicando fora nem com `Esc` (equivale a "Recusar" para evitar aprovação acidental).
```
╭────────────────────────────────────────╮
│  🛡  Permitir que "Pixel 8" se conecte? │
│                                        │
│  Modelo: Pixel 8  ·  IP: 192.168.0.23  │
│  Só aprove se for o seu aparelho.      │
│                                        │
│  ◔ Expira em 27 s                       │
│                                        │
│  [ Recusar ]  [ Só desta vez ] [Permitir sempre] │
╰────────────────────────────────────────╯
```
- Foco inicial em **Recusar**. "Permitir sempre" é o botão primário (gradiente), mas **não** é o foco inicial.
- Barra/anel de contagem regressiva de 30 s; ao zerar, fecha como "não aprovado".
- Se a janela estiver minimizada, **trazer ao primeiro plano** e emitir notificação do Windows ("Novo aparelho quer se conectar").
- Dois pedidos simultâneos: o segundo é recusado automaticamente (`busy`).

### 2.8.2 Assistente de primeiro uso (`SetupWizard`)
Modal de 4 passos com indicador de progresso; cada passo mostra o estado atual e o botão **Corrigir**; pode ser pulado e reaberto em Ajustes.
1. **Rede:** regra de firewall (UAC) e descoberta.
2. **Câmera virtual:** registrar a DLL e a câmera (UAC).
3. **Microfone virtual** (opcional): explica modo de teste/HVCI e oferece "Pular por enquanto".
4. **ADB** (opcional): localizar/baixar.
No final: "Tudo pronto — escaneie o QR com o app do celular".

### 2.8.3 Demais diálogos
| Diálogo | Tipo | Texto-chave |
|---|---|---|
| Regenerar QR | `AlertDialog` | `desktop.connection.regenerateConfirm` |
| Remover dispositivo confiável | `AlertDialog` (foco em Cancelar) | `desktop.trusted.removeConfirm` |
| Baixar platform-tools | `Dialog` (consentimento) | `desktop.adb.downloadConsent` |
| Versão incompatível | `Dialog` | `common.errors.version_mismatch.*` (diz **qual lado atualizar**, com as versões) |
| Trocar modo dual (recria câmeras) | `AlertDialog` | `desktop.camera.dualModeWarning` |
| Reiniciar servidor (porta) | Banner no painel | `desktop.settings.portRestart` |

### 2.8.4 Toasts (`Sonner`)
Canto inferior esquerdo (não cobre o preview), 4 s (erros 8 s, com ação). Usados para: copiado, dispositivo conectado/desconectado, erro de decodificação recuperado ("Imagem retomada"), configuração rejeitada pelo celular. **Não** usar toast para erros que bloqueiam o fluxo (usar estado do preview ou diálogo).

## 2.9 Atalhos
| Atalho | Ação |
|---|---|
| `M` | Espelhar |
| `R` | Girar 90° |
| `C` | Alternar câmera (traseira/frontal) |
| `S` | Trocar principal/miniatura (PiP) |
| `F` / `F11` | Tela cheia do preview |
| `Esc` | Sai da tela cheia; fecha diálogos (exceto aprovação) |
| `Shift+M` | Mudo do microfone do celular |
| `Ctrl+,` | Abrir aba Ajustes |
| `Ctrl+1…4` | Abas Conexão, Câmera, Áudio, Ajustes |
Atalhos de tecla única só valem quando o foco **não** está em campo de texto.

## 2.10 Estados vazios e de erro (tabela de referência)

| Situação | Onde aparece | Mensagem (chave) | Ação |
|---|---|---|---|
| Sem celular | Preview | `desktop.empty.*` | QR / USB |
| Porta ocupada | Rodapé + banner | `desktop.errors.portInUse` | Usar porta alternativa |
| Firewall bloqueando | Passo 1 do assistente / rodapé | `desktop.errors.firewall` | Corrigir (UAC) |
| ADB ausente | Aba Conexão + rodapé | `desktop.adb.missing` | Baixar |
| Aparelho USB não autorizado | Linha USB | `desktop.adb.unauthorized` | Instrução |
| Câmera virtual não registrada | Rodapé / assistente | `desktop.vcam.notRegistered` | Corrigir |
| Erro de decodificação | Toast + preview "Reconectando" breve | `desktop.errors.decode` | Automático (pede keyframe) |
| Celular caiu | Preview `reconnecting` | `common.state.reconnecting` | Automático; botão "Parar de esperar" |
| Dispositivo revogado | Preview `error` | `common.errors.revoked` | — |
| Versão incompatível | Diálogo | `common.errors.version_mismatch.*` | Instrução |

## 2.11 Árvore de componentes e estado (guia para implementação)

```
desktop/src/
├── app/
│   ├── App.tsx                  # providers (i18n, store, tooltip), SetupWizard, Toaster
│   ├── Shell.tsx                # grade: Titlebar · (PreviewArea | SidePanel) · HealthBar
│   └── Titlebar.tsx             # arrasto, StatusChip, LanguageMenu, controles da janela
├── features/
│   ├── preview/
│   │   ├── PreviewArea.tsx      # escolhe o estado (§2.5.2)
│   │   ├── PreviewCanvas.tsx    # canvas imperativo (fora do React por quadro)
│   │   ├── PreviewOverlays.tsx  # chips + Toolbar
│   │   ├── Toolbar.tsx
│   │   └── DualLayout.tsx       # PiP arrastável / duas molduras
│   ├── connection/              # ConnectionTab, QRCard, UsbList, TrustedList, DeviceCard
│   ├── approval/                # ApprovalDialog
│   ├── camera/                  # CameraTab (SettingRows)
│   ├── audio/                   # AudioTab, LevelMeter
│   ├── settings/                # SettingsTab, SetupWizard
│   └── health/                  # HealthBar, HealthIndicator, popovers de correção
├── store/                       # zustand (slices abaixo)
├── hooks/                       # useTauriEvent, useShortcuts, useReducedMotion
└── lib/                         # tauri.ts (invoke/listen tipados), preview-player.ts (estratégia do S5)
```

**Slices do store** (seletores estreitos):
| Slice | Conteúdo | Atualizado por |
|---|---|---|
| `connection` | estado (`waiting`/`pairing`/`connected`/`reconnecting`/`error`), aparelho, transporte, códigos de erro | `session://connection` |
| `cameraState` | espelho do `STATE` do celular (`rev`, `camera`, `video`, `presentation`, `audio`) + otimismo pendente | `session://state` + ações da UI |
| `stats` | fps, bitrate, RTT, descartes, nível de áudio | `session://stats` (≤ 2 Hz) |
| `trust` | pedido pendente, lista de confiáveis | `trust://pending`, comandos |
| `usb` | aparelhos ADB | `adb://devices` |
| `health` | cinco indicadores | `health://changed` |
| `settings` | preferências persistidas | comandos de settings |
| `ui` | aba ativa, tela cheia, diálogos abertos | local |

Regras: ações da UI chamam `invoke` e **só confirmam** quando o evento correspondente volta; para controles de câmera, aplicar atualização otimista e reconciliar pelo `rev` (Fase 6).
