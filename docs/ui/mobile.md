# Mobile — Especificação de UI (§3 do `ui-spec`)

Pré-requisito: `docs/ui-spec.md` (tokens, glass, componentes, regras de desempenho). Textos em `docs/ui/copy.md`. Estados e códigos em `docs/protocol.md` §6 e §12.

**Restrições:** React 19 + Tailwind 3.4 + shadcn/ui; **sem framer-motion** (só CSS); Capacitor 8.5; UI travada em **retrato**; alvos ≥ 44 px; fonte ≥ 16 px no corpo.

---

## 3.1 Modelo de navegação

Sem barra de abas. Pilha simples de telas em tela cheia + **bottom sheets** para configurações e ajuda. A tela exibida é **derivada do estado da conexão** (máquina de estados do protocolo §12) e das permissões:

| Condição | Tela |
|---|---|
| Permissões obrigatórias (câmera, microfone) ainda não concedidas | **Permissões** |
| `Idle` | **Home** |
| Usuário tocou em Escanear | **Scan** |
| Usuário tocou em Inserir manualmente | **Manual** |
| `Connecting` (primeira tentativa) | **Conectando** |
| `Pending` | **Aguardando aprovação** |
| `Streaming` | **Transmitindo** |
| `Reconnecting` | **Reconectando** (sobre a tela Transmitindo, ver §3.8) |
| Erro definitivo (`revoked`, `version_mismatch`, `busy`, `not_approved`, `bad_token`) | **Erro definitivo** (§3.10) |

```
Permissões ─► Home ─► Scan ─┐
               │  ╰► Manual ┤
               ▼            ▼
          [PCs conhecidos]  Conectando ─► Aguardando aprovação ─► Transmitindo ◄─► Reconectando
               ▲                                   │                    │
               ╰──────────── Erro definitivo ◄─────┴────────────────────╯   (Parar → Home)
```

**Botão voltar (Android, via `@capacitor/app`):**
| Tela | Comportamento |
|---|---|
| Home | Sai do app (minimiza) |
| Scan / Manual | Volta à Home |
| Conectando / Aguardando | Cancela a tentativa e volta à Home |
| **Transmitindo** | Se "Continuar transmitindo com a tela apagada" estiver **ligado**: minimiza o app (a transmissão segue). Se **desligado**: abre diálogo "Parar a transmissão?" (Cancelar/Parar) |
| Sheet aberto | Fecha o sheet |

## 3.2 Layout base

- **Edge-to-edge**: conteúdo atrás da barra de status e de navegação, com `padding` de *safe-area* (`env(safe-area-inset-*)`). Barra de status transparente com ícones claros; barra de navegação na cor `bg`.
- Margens laterais 20 px; espaçamento vertical entre blocos 16–24 px.
- Uma ação principal por tela, **na parte de baixo** (alcance do polegar), 52–56 px de altura, largura total.
- Fundo: gradiente `bg` (§1.3); `reduce-glass` ligado automaticamente em `Streaming`.

## 3.3 Tela: Permissões (primeira execução)

```
┌──────────────────────────────┐
│          ◉  (logo)           │
│  Precisamos de algumas       │
│  permissões                  │
│                              │
│ ┌──────────────────────────┐ │
│ │ 📷 Câmera           [ OK]│ │
│ │ Para enviar a imagem ao  │ │
│ │ PC.                      │ │
│ └──────────────────────────┘ │
│ ┌──────────────────────────┐ │
│ │ 🎤 Microfone  [Permitir] │ │
│ └──────────────────────────┘ │
│ ┌──────────────────────────┐ │
│ │ 🔔 Notificações [Permitir]│ │
│ │ Mantém a transmissão ativa│ │
│ └──────────────────────────┘ │
│                              │
│ [        Continuar         ] │
└──────────────────────────────┘
```
- Três cartões (`surface-1`) com ícone, título, motivo e botão **Permitir** (vira `✓ Concedida` em `ok`).
- **Câmera é obrigatória** (sem ela, "Continuar" fica desabilitado, com motivo). **Microfone e notificações são recomendados**: o usuário pode continuar sem eles (aviso de que o áudio/serviço ficarão limitados).
- **Negada permanentemente** (o sistema não pergunta mais): o botão do cartão vira **Abrir configurações do Android** (`openSettings`).
- Reaparece como cartão informativo na Home se a permissão for revogada depois.

## 3.4 Tela: Home (desconectado)

```
┌──────────────────────────────┐
│ ◉ LocalCamera            ⚙   │
│                              │
│   ( ○ ) Desconectado         │
│   Pronto para conectar       │
│                              │
│ ┌──────────────────────────┐ │
│ │ 💻 Reconectar a DESKTOP-X│ │   ← só se houver PC conhecido
│ │ visto há 2 h       [ → ] │ │
│ └──────────────────────────┘ │
│                              │
│ ┌──────────────────────────┐ │
│ │ 🔌 Prefere cabo? Conecte │ │
│ │ o USB e toque em         │ │
│ │ Conectar no app do PC.   │ │
│ └──────────────────────────┘ │
│                              │
│ [  ▣  Escanear QR code     ] │   ← primário
│ [   Inserir manualmente    ] │   ← secundário
│ Meus PCs (2)  ›              │
└──────────────────────────────┘
```
- `⚙` abre **Configurações** (sheet, §3.11).
- **Reconectar** usa a **cadeia** do protocolo (mDNS → últimos IPs → pedir QR) e leva à tela **Reconectando**; com vários PCs, mostra o último usado e o link "Meus PCs".
- **Meus PCs** abre uma lista (`DeviceRow`: nome, "visto há…", **Conectar**, **Esquecer** com confirmação).
- Cartão de aviso quando faltam permissões ou o Wi-Fi está desligado ("Sem rede. Conecte-se ao Wi-Fi ou use o cabo USB.").

## 3.5 Tela: Scan (QR)

```
┌──────────────────────────────┐
│ ←  Escanear QR code      🔦  │
│ ┌──────────────────────────┐ │
│ │                          │ │
│ │       ┌ ─ ─ ─ ─ ┐        │ │   ← moldura de mira com cantos
│ │       │         │        │ │
│ │       └ ─ ─ ─ ─ ┘        │ │
│ │                          │ │
│ └──────────────────────────┘ │
│ Aponte para o QR code        │
│ exibido no PC                │
│ [ Inserir manualmente ]      │
└──────────────────────────────┘
```
- Câmera ocupa a tela (por trás da UI, via plugin de scanner); lanterna (`🔦`) alterna.
- Aceita **somente** URIs `localcamera://connect...` válidas. QR inválido: faixa `danger` "Este QR code não é do LocalCamera" por 3 s, sem sair da tela.
- URI válida → vibração curta (haptics opcional) → **Conectando**.
- Permissão de câmera negada: estado com texto + botão **Abrir configurações**.
- O QR também abre o app se escaneado pela câmera nativa do Android (deep link `appUrlOpen`) — cai direto em **Conectando**.

## 3.6 Tela: Manual

Campos (shadcn `Input`, teclado apropriado, validação inline ao sair do campo):
| Campo | Tipo | Validação | Padrão |
|---|---|---|---|
| Endereço do PC | texto (IP ou nome) | IPv4 válido ou hostname | — |
| Porta | numérico | 1–65535 | 48650 |
| Código de pareamento | texto, 32 hex | `^[0-9a-fA-F]{32}$` | — |
Atalho: botão **Colar link** (lê a área de transferência e preenche a partir de uma URI `localcamera://`). Botão primário **Conectar** (desabilitado até todos os campos serem válidos).

## 3.7 Telas: Conectando e Aguardando aprovação

**Conectando:** `Loader2` grande, "Conectando a {PC}…", lista das etapas só em reconexão (§3.8), botão **Cancelar**. Timeout de 10 s → erro "Não foi possível conectar" com **Tentar de novo** / **Escanear QR**.

**Aguardando aprovação:**
```
┌──────────────────────────────┐
│                              │
│          ╭───────╮           │
│          │  27   │  ← CountdownRing (30 s)
│          ╰───────╯           │
│     Aprove no PC             │
│  Um aviso apareceu no        │
│  computador. Toque em        │
│  "Permitir sempre".          │
│                              │
│  [        Cancelar         ] │
└──────────────────────────────┘
```
Ao zerar sem resposta → erro `not_approved`.

## 3.8 Tela: Transmitindo (a principal)

```
┌──────────────────────────────┐
│ ( ● Transmitindo )  💻 DESKTOP-X │ ← header: StatusChip + nome do PC
│ [Wi-Fi]                       ⚙   │
│                              │
│        ╭───────────╮         │
│        │  ◉  LIVE  │         │   ← anel pulsando (CSS, só opacity/transform)
│        │ 1080p · 30│         │
│        ╰───────────╯         │
│                              │
│ ┌─────────┐ ┌─────────┐      │
│ │ Bitrate │ │Latência │      │   ← StatTile 2×2
│ │ 8,0 Mbps│ │  12 ms  │      │
│ └─────────┘ └─────────┘      │
│ ┌─────────┐ ┌─────────┐      │
│ │ Quadros │ │ Câmera  │      │
│ │ 0 perdid│ │ Traseira│      │
│ └─────────┘ └─────────┘      │
│                              │
│ ( Traseira | Frontal | Dupla)│   ← SegmentedControl 48 px
│ [🎤 Mic] [⇋ Espelho] [⟳ Girar]│  ← BigToggle 56 px
│                              │
│ [        ■  Parar          ] │   ← destrutivo, largura total
└──────────────────────────────┘
```
- **Sem prévia da câmera** nesta versão (backlog). O anel e o texto deixam claro que está no ar.
- **Dupla**: opção desabilitada com motivo quando `concurrent=false` (texto `small` sob o controle). No modo dual, o tile "Câmera" mostra "Traseira + Frontal".
- Alternadores refletem o `STATE` (converge com o PC; se o PC mudar, a UI muda em < 1 s). Troca de câmera/resolução mostra spinner breve no controle.
- **Parar**: toque único, sem confirmação (reiniciar é fácil); encerra serviço e volta à Home. Protegido de toque acidental por espaçamento (≥ 24 px de outros alvos).
- Chip de aviso `warn` se: fila de envio alta ("Rede lenta — qualidade reduzida"), superaquecimento ("O aparelho está esquentando"), tela com economia de bateria agressiva ("O sistema pode pausar a transmissão" → link para Ajuda de bateria).
- Em `Streaming`: `reduce-glass` ligado, animações só no anel; quando a tela apaga/app em segundo plano, **pausar animações** (a notificação é a interface).
- **Reconectando** (sobreposição, `Reconnecting`): painel `overlay` sobre a tela com as etapas em lista de 3 itens (✓ concluído / ◌ em andamento / ○ pendente): "Procurando o PC na rede…" → "Tentando os últimos endereços…" → "Não encontramos o PC" (com **Escanear QR** e **Cancelar**). A transmissão fica pausada visualmente, mas o serviço segue tentando com backoff (1, 2, 4, 8 s).

## 3.9 Tela: PCs conhecidos
Lista de `DeviceRow` (ícone `Monitor`, nome, "visto há…"), ações **Conectar** e **Esquecer** (`AlertDialog`: "Esquecer *{PC}*? Será preciso escanear o QR e aprovar de novo."). Vazio: `EmptyState` "Nenhum PC pareado ainda" + botão Escanear.

## 3.10 Telas de erro definitivo

Layout único (`EmptyState` em tela cheia): ícone grande na cor do erro, título, explicação, 1 ação primária e 1 secundária.

| Código | Título | Ação primária | Ação secundária |
|---|---|---|---|
| `bad_token` | "Código inválido" | Escanear QR code | Inserir manualmente |
| `not_approved` | "Conexão não aprovada" | Tentar de novo | Voltar |
| `revoked` | "Este aparelho foi removido do PC" (apaga a chave local) | Escanear QR code | Voltar |
| `busy` | "O PC já está conectado a outro celular" | Tentar de novo | Voltar |
| `version_mismatch` | "Atualize o app" — texto diz **qual lado** (celular ou PC) e mostra as duas versões | Voltar | — |
| `timeout` / `protocol_error` / `bad_audio_ticket` | "Falha na comunicação" (a reconexão automática tenta antes de mostrar esta tela) | Tentar de novo | Voltar |
| Sem rede | "Sem rede" | Abrir configurações de Wi-Fi | Usar USB (instrução) |

## 3.11 Configurações (bottom sheet de 85% da altura)

| Seção | Item | Controle | Padrão | Descrição (`small`) |
|---|---|---|---|---|
| Transmissão | **Continuar transmitindo com a tela apagada** | `Switch` | ligado | "Mantém a câmera ativa em segundo plano com uma notificação." |
| Transmissão | **Manter a tela ligada** | `Switch` | desligado | "A tela não apaga enquanto o app está aberto transmitindo." |
| Transmissão | **Qualidade padrão** | `Select` | 1080p · 30 | Só opções suportadas (`caps`) |
| Áudio | **Fonte do microfone** | `Select` | Microfone | Microfone / Comunicação de voz |
| Geral | **Idioma** | `Select` | Sistema | Sistema / Português / English |
| Ajuda | **Otimização de bateria** | Linha → sheet de ajuda | — | Abre §3.12 |
| Ajuda | **PCs conhecidos** | Linha | — | Abre §3.9 |
| Sobre | Versão do app e do protocolo, licenças | Linha | — | |
Se **ambos** os interruptores de transmissão estiverem desligados, mostrar aviso `warn`: "A transmissão para quando você sair do app."

## 3.12 Ajuda de bateria (sheet)
Texto curto explicando que alguns fabricantes encerram apps em segundo plano; **botão "Abrir configurações de bateria"** (intent do sistema) e instrução genérica ("Defina o LocalCamera como *Sem restrições*"). Texto neutro: não cita marcas específicas além de dizer "alguns fabricantes".

## 3.13 Android: tema, ícone e notificação

| Item | Especificação |
|---|---|
| Tema/barras | `Theme` escuro, barra de status transparente com ícones claros, barra de navegação `#070B14`, edge-to-edge |
| Splash | API de splash do Android 12+ (confirmar na doc): fundo `#070B14`, ícone centralizado |
| Ícone do app | Adaptativo (primeiro plano: lente + arco; fundo: gradiente accent→blue) + ícone monocromático (Android 13+) |
| Canal de notificação | id `streaming`, importância **baixa** (sem som), nome localizado ("Transmissão") |
| Notificação do serviço | Título "LocalCamera — transmitindo"; texto "{PC} · 1080p30 · Wi-Fi"; ícone pequeno monocromático; **ação "Parar"**; toque abre o app; atualiza o texto em reconexão ("Reconectando…") |
| Idioma nativo | `res/values/strings.xml` (en) e `res/values-pt-rBR/strings.xml` (chaves em `docs/ui/copy.md`) |

## 3.14 Componentes e estrutura

```
mobile/src/
├── App.tsx                     # providers (i18n, store), roteador por estado
├── screens/
│   ├── PermissionsScreen.tsx
│   ├── HomeScreen.tsx
│   ├── ScanScreen.tsx · ManualScreen.tsx
│   ├── ConnectingScreen.tsx · PendingScreen.tsx
│   ├── StreamingScreen.tsx · ReconnectingOverlay.tsx
│   ├── KnownPcsScreen.tsx
│   ├── ErrorScreen.tsx         # recebe o código; usa a tabela §3.10
│   └── sheets/ SettingsSheet.tsx · BatteryHelpSheet.tsx
├── components/                 # específicos (CountdownRing, BigToggle, LiveRing)
├── plugin/                     # wrapper TS do LocalCameraPlugin + hooks (useConnection, useStreamState)
├── store/                      # zustand: connection, streamState, settings, permissions
└── main.tsx
```

**Mapeamento plugin → UI**
| Evento/consulta do plugin | Efeito na UI |
|---|---|
| `connectionChanged { state, transport, pc }` | Troca de tela (tabela §3.1) |
| `stateChanged { rev, camera, video, presentation, audio }` | Atualiza controles e tiles |
| `stats { bitrate, rttMs, dropped, queueMs }` (≤ 2 Hz) | Atualiza `StatTile`s e chips de aviso |
| `error { code }` | `ErrorScreen` ou toast, conforme §3.10 |
| `getCapabilities()` | Habilita/desabilita Dupla, resoluções, fps |
| `appUrlOpen` (deep link) | Preenche a conexão e vai a **Conectando** |

## 3.15 Desempenho e bateria (mobile)
- Durante a transmissão: sem blur, sem animações além do anel, `will-change` apenas no anel; atualizar estatísticas a ≤ 2 Hz.
- Não manter WebView renderizando em segundo plano: pausar timers de UI quando `document.hidden`.
- Nenhuma imagem raster pesada; SVG inline.
- Medir consumo (bateria e temperatura) após 30 min na Fase 9 e registrar em `docs/perf/v1.md`.
