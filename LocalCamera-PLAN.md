# LocalCamera — Plano mestre (revisão 2)

> Este é o **índice e a visão geral** do projeto. Os detalhes estão em arquivos separados:
> - `docs/protocol.md` — especificação do protocolo (fonte da verdade)
> - `docs/ui-spec.md` — **design system** (tokens, glass, tipografia, componentes, estados), marca, desempenho, i18n e acessibilidade
> - `docs/ui/desktop.md` — interface do desktop (janela, preview, painel, diálogos, atalhos, estados)
> - `docs/ui/mobile.md` — interface do celular (telas, navegação, erros, Android)
> - `docs/ui/copy.md` — todos os textos em pt-BR e en, com as chaves de i18n
> - `docs/phases/fase-N-*.md` — **um arquivo por fase**, com tarefas, testes de aceitação e critério de saída
>
> Regra de ouro: **não invente APIs.** Onde um documento diz "confirmar na documentação", consulte a documentação oficial (Microsoft Learn, developer.android.com, tauri.app, capacitorjs.com) antes de escrever código.

## 0. Como executar com um modelo de contexto pequeno (Qwen 27B / 200k)

O trabalho é entregue em **cartões de tarefa** pequenos e autossuficientes (`docs/tasks/fase-N/<ID>.md`), com um **`PROGRESS.md`** na raiz que o agente atualiza a cada tarefa. O `AGENTS.md` é curto de propósito. Fluxo: ler `PROGRESS.md` → abrir **só** o cartão da próxima tarefa → executar → verificar → commit → atualizar `PROGRESS.md`. Prompts prontos: `docs/tasks/PROMPTS.md`. Fatos recorrentes: `docs/CHEATSHEET.md`.

Os cartões das **Fases 0 e 1** estão prontos. Os das Fases 2–9 dependem dos vereditos dos spikes (S1, S2, S3, S5 e S4) e são gerados **depois** de cada fase anterior fechar; os arquivos de fase (`docs/phases/`) continuam sendo a especificação e o plano de testes.

### 0.1 Como o agente deve usar os documentos de fase

1. Leia `AGENTS.md`, depois este arquivo, depois `docs/protocol.md`.
2. Trabalhe **uma fase por vez**: abra apenas o arquivo da fase atual (e o que ele mandar ler). Não carregue as outras.
3. Cada fase tem tarefas com ID (`F3-T05`), testes de aceitação (`AT-3.04`) e um "Pronto quando". Marque o checklist no próprio arquivo ao concluir.
4. **Spikes** (`S1`, `S2`, `S3`, `S4`, `S5`) são experimentos curtos que terminam com um relatório em `docs/spikes/` e uma decisão registrada em `docs/adr/`. O código das fases seguintes **segue o veredito do spike**, não a suposição deste plano.

---

## 1. Visão geral

**LocalCamera** transforma um celular Android em webcam e microfone para um PC com Windows 11, por Wi-Fi ou cabo USB, no estilo do Iriun, com código aberto.

Três peças:

1. **App Android** (`mobile/`): captura câmera e microfone, codifica e envia.
2. **App Desktop** (`desktop/`): recebe, decodifica, mostra preview, controla o celular e alimenta os dispositivos virtuais.
3. **Dispositivos virtuais no Windows**: uma câmera virtual (DLL COM, Media Foundation) e um microfone virtual (driver de áudio).

Meta de qualidade: **1080p a 30 fps** por padrão; latência de ponta a ponta abaixo de ~150 ms por USB e ~250 ms por Wi-Fi.

---

## 2. O que mudou nesta revisão

| # | Melhoria | Onde foi tratada |
|---|---|---|
| 1 | Áudio e vídeo disputando o mesmo socket | **Dois sockets** (principal: controle+vídeo; áudio: dedicado) + fila com prioridade. `docs/protocol.md` §3 e §11 |
| 2 | CameraX fixado cedo demais | Interface `CameraBackend` com duas implementações possíveis (CameraX e Camera2); o **spike S3** decide. Fase 0 e Fase 7 |
| 3 | Preview com WebCodecs não validado | **Spike S5** compara WebCodecs × preview reduzido gerado pelo Rust. Fase 0 e Fase 3 |
| 4 | Reconexão dependia de IP fixo | **mDNS no MVP** (Fase 5) + **aprovação no PC na primeira conexão** e chave por dispositivo (Fases 1 e 3) |
| 5 | Driver de áudio: risco e possível reaproveitamento | Spike **S4 movido para o início da Fase 8**, com pesquisa de drivers existentes e checagem de Secure Boot/HVCI/anti-cheat |
| 6 | Documento único e sem marcos | Um arquivo por fase, checklists, testes de aceitação e **marcos de entrega M1–M5** |
| — | Menores | UI com estatísticas limitadas a 2 Hz e preview fora do ciclo do React; compatibilidade de versões do protocolo e dos apps; **CI local** com hook de `pre-push` (Fase 0) |

---

## 3. Decisões já tomadas (não reabrir sem motivo)

| Tema | Decisão |
|---|---|
| Nome | LocalCamera |
| Uso | Só uso próprio. Driver de áudio em **modo de teste** do Windows (ou driver de terceiros já assinado, se o S4 permitir) |
| SO do PC | **Somente Windows 11** (API `MFCreateVirtualCamera`) |
| Casca do desktop | **Tauri 2** (backend Rust + frontend React) |
| Frontend desktop | React 19, TypeScript, Vite, Tailwind CSS 3.4 (tokens custom, dark glassmorphism), framer-motion, lucide-react, clsx, tailwind-merge, **shadcn/ui** |
| Frontend mobile | React 19, TypeScript, Vite, Tailwind CSS 3.4, Capacitor 8.5 + `@capacitor/app`, lucide-react, clsx, tailwind-merge, **shadcn/ui**. **Sem framer-motion.** |
| Captura mobile | **Plugin nativo Kotlin próprio** com MediaCodec (H.264). Backend de câmera (**CameraX ou Camera2**) definido pelo spike S3, atrás de uma interface |
| Rede | **Dois sockets TCP** na mesma porta: principal (controle + vídeo) e áudio |
| Conexão | Wi-Fi (QR code) e USB (`adb reverse`); reconexão por **mDNS** |
| Pareamento | QR com token de pareamento → **aprovação no PC** → chave por dispositivo. Sem criptografia do tráfego (risco aceito, uso próprio) |
| Qualidade padrão | 1080p / 30 fps |
| Câmera dupla | Só em aparelhos que suportem. No PC, o usuário escolhe entre **duas webcams separadas** ou **uma webcam com picture-in-picture** |
| Controle | Os dois lados controlam; o último comando recebido vence. O celular é dono do estado |
| Áudio | Microfone do celular → microfone virtual no Windows (driver; ver Fase 8) |
| Segundo plano (celular) | O usuário escolhe nas configurações: serviço em primeiro plano e/ou manter tela ligada |
| Idiomas | Português (Brasil) e inglês desde o início |
| Layout do desktop | Janela única: preview grande ao centro + painel lateral |
| Cor de destaque | Azul/ciano |
| Repositório | Monorepo pnpm workspaces: `desktop/`, `mobile/`, `packages/shared` |
| Licença | MIT para o código próprio (verificar licenças das dependências) |

---

## 4. Arquitetura

```
┌──────────────── Celular Android ────────────────┐        ┌──────────────────── PC Windows 11 ────────────────────┐
│ React UI (Capacitor WebView)                    │        │ React UI (Tauri WebView2)                              │
│        │ plugin                                 │        │        │ comandos / eventos / Channel                  │
│ LocalCameraPlugin (Kotlin)                      │        │ Backend Rust (Tauri)                                   │
│  ├ CameraBackend (CameraX | Camera2)  ←  S3     │ TCP #1 │  ├ Servidor TCP (principal + áudio) + handshake        │
│  ├ VideoEncoder x1/x2 (MediaCodec H.264)        │◄──────►│  ├ Confiança: aprovação, chaves por dispositivo        │
│  ├ AudioCapture (AudioRecord, PCM)              │ TCP #2 │  ├ Decodificador H.264 (Media Foundation, GPU)         │
│  ├ PrimaryConnection / AudioConnection          │◄──────►│  ├ Composição (rotação, espelho, PiP) → NV12 1080p     │
│  ├ Discovery (NSD / mDNS)                       │  mDNS  │  ├ Anúncio mDNS (_localcamera._tcp)                    │
│  └ StreamingService (foreground service)        │◄ ─ ─ ─►│  ├ Cliente do canal da câmera virtual (pipe / S2)      │
└─────────────────────────────────────────────────┘        │  ├ Gerenciador de ADB (reverse, deep link)             │
          Wi-Fi  ou  USB (adb reverse)                     │  └ Áudio → microfone virtual (Fase 8)                  │
                                                           └───────┬───────────────────────────────┬───────────────┘
                                                                   │ frames NV12                   │ PCM
                                                    ┌──────────────▼───────────┐     ┌─────────────▼─────────────┐
                                                    │ virtualcam/ (DLL COM)    │     │ microfone virtual         │
                                                    │ carregada pelo Frame     │     │ (driver; decisão no S4)   │
                                                    │ Server do Windows        │     │                           │
                                                    └──────────────┬───────────┘     └─────────────┬─────────────┘
                                                                   ▼                               ▼
                                                  Zoom, Teams, Discord, Chrome, OBS,       Zoom, Discord, OBS, ...
                                                  NVIDIA Broadcast (entrada de câmera)
```

O **celular é dono do estado** da câmera (resolução, câmera ativa etc.); o **PC é o servidor de rede**. O celular sempre conecta no PC.

---

## 5. Estrutura do repositório

```
LocalCamera/
├── AGENTS.md
├── LocalCamera-PLAN.md
├── package.json                    # raiz; scripts: lint, typecheck, test, ci
├── pnpm-workspace.yaml             # desktop, mobile, packages/*
├── Cargo.toml                      # workspace Rust: crates/*, tools/*, desktop/src-tauri
├── tsconfig.base.json
├── .githooks/pre-push              # roda scripts/ci.ps1 -Fast
├── desktop/
│   ├── src/                        # React 19
│   │   ├── app/                    # layout da janela, titlebar custom
│   │   ├── features/               # preview, connection, camera, audio, settings, health, trust
│   │   ├── hooks/ · lib/ · store/  # zustand; tauri.ts; player do preview
│   │   └── main.tsx
│   └── src-tauri/src/
│       ├── commands.rs · settings.rs · main.rs
│       ├── session/                # servidor TCP, handshake, estado
│       ├── trust/                  # aprovação, chaves, dispositivos confiáveis
│       ├── discovery/              # anúncio mDNS
│       ├── decode/                 # Media Foundation H.264
│       ├── compose/                # rotação, espelho, PiP, escala → NV12
│       ├── preview/                # estratégia decidida no S5
│       ├── vcam/                   # registro + cliente do canal de frames
│       ├── adb/
│       └── audio/
├── mobile/
│   ├── src/                        # React 19 (screens/, plugin/, main.tsx)
│   └── android/app/src/main/java/com/localcamera/app/
│       ├── MainActivity.kt
│       ├── plugin/LocalCameraPlugin.kt
│       ├── service/StreamingService.kt        # LifecycleService, foreground
│       ├── session/Session.kt · StateReducer.kt
│       ├── camera/CameraBackend.kt            # interface
│       ├── camera/CameraXBackend.kt · Camera2Backend.kt   # conforme S3
│       ├── video/VideoEncoder.kt
│       ├── audio/AudioCapture.kt
│       ├── net/PrimaryConnection.kt · AudioConnection.kt · SendQueue.kt
│       ├── net/Discovery.kt                   # NsdManager
│       ├── trust/PairingStore.kt              # deviceKey, PCs conhecidos
│       └── protocol/ProtocolCodec.kt
├── packages/shared/                # @localcamera/shared
│   ├── src/protocol/               # tipos TS das mensagens JSON, constantes
│   ├── src/i18n/                   # i18next + pt-BR.json + en.json
│   ├── src/ui/                     # shadcn/ui compartilhados (SEM framer-motion)
│   ├── src/lib/cn.ts
│   └── tailwind-preset.ts
├── crates/lc-protocol/             # parser/serializador Rust
├── tools/mock-sender/              # CLI Rust: finge ser o celular
├── virtualcam/                     # DLL C++ da câmera virtual
├── driver/audio/                   # driver de áudio (se o S4 decidir por driver próprio/fork)
├── scripts/                        # ci.ps1, dev-setup.ps1
└── docs/
    ├── protocol.md · protocol-vectors.json · composition.md
    ├── ui-spec.md                  # design system + regras compartilhadas
    ├── ui/desktop.md · ui/mobile.md · ui/copy.md · ui/screens/   # capturas de tela de referência
    ├── phases/fase-0 … fase-9
    ├── spikes/                     # relatórios S1–S5
    └── adr/                        # decisões de arquitetura
```

**Regra do `packages/shared`:** nada ali importa `framer-motion`. Componentes animados do desktop ficam em `desktop/src`.

---

## 6. Pré-requisitos de ambiente

- **Windows 11** com Visual Studio 2022 (workload "Desktop development with C++"), Windows SDK recente e **WDK** (necessário se o S4 levar a um driver próprio).
- Rust estável (toolchain MSVC), Tauri CLI v2, Node (versão exigida pelo Capacitor 8.5) e pnpm.
- Android Studio, Android SDK e o JDK exigido pelo Capacitor 8.5 (**confirmar versões na documentação**).
- Um celular Android real com depuração USB. Para a câmera dupla, o aparelho precisa suportar câmeras simultâneas (confirmado no S3).
- `ffmpeg` (só para gerar o arquivo de teste do mock-sender) e `adb`.
- **Para a Fase 8:** VM ou PC de teste para o driver. Ver as exigências de Secure Boot, modo de teste e HVCI na Fase 8.

---

## 7. Marcos e fases

| Marco | Fases | Entrega | Tag |
|---|---|---|---|
| **M1** | 0, 1, 2, 3, 4 | **"Webcam por Wi-Fi funcionando"**: QR → aprovação no PC → vídeo 1080p30 → câmera virtual "LocalCamera" funcionando nos apps e no NVIDIA Broadcast | `v0.1.0` |
| **M2** | 5, 6 | **"USB, reconexão e controle"**: cabo USB, mDNS, controles nos dois lados, espelho/rotação, bitrate adaptativo | `v0.2.0` |
| **M3** | 7 | **"Câmera dupla"** (modo separado e PiP) | `v0.3.0` |
| **M4** | 8 | **"Microfone virtual"** | `v0.4.0` |
| **M5** | 9 | **Versão 1.0**: segundo plano, polimento, instalação e documentação | `v1.0.0` |

| Fase | Arquivo | Tamanho |
|---|---|---|
| 0 — Fundação e spikes (S1, S2, S3, S5) | `docs/phases/fase-0-fundacao-e-spikes.md` | M |
| 1 — Protocolo e rede | `docs/phases/fase-1-protocolo-e-rede.md` | M |
| 2 — Mobile: transmissão mínima | `docs/phases/fase-2-mobile-transmissao-minima.md` | G |
| 3 — Desktop: UI, decodificação e preview | `docs/phases/fase-3-desktop-ui-e-preview.md` | G |
| 4 — Câmera virtual | `docs/phases/fase-4-camera-virtual.md` | G |
| 5 — USB e descoberta (mDNS) | `docs/phases/fase-5-usb-e-descoberta.md` | M |
| 6 — Controle bidirecional | `docs/phases/fase-6-controle-bidirecional.md` | M |
| 7 — Câmera dupla | `docs/phases/fase-7-camera-dupla.md` | G |
| 8 — Áudio (spike S4 + microfone virtual) | `docs/phases/fase-8-audio.md` | G |
| 9 — Polimento e instalação | `docs/phases/fase-9-polimento-e-instalacao.md` | M |

Tamanhos: **P** pequena, **M** média, **G** grande.

---

## 8. Qualidade, testes e CI local

**Scripts (criados na Fase 0):**
- `pnpm lint`, `pnpm typecheck`, `pnpm test`, `cargo test --workspace`.
- `scripts/ci.ps1` (alias `pnpm ci`): instala com lockfile congelado → lint → typecheck → testes JS → `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` → vetores do protocolo → testes e lint do Android (`./gradlew testDebugUnitTest lintDebug`, pulável com `-SkipAndroid`).
- `.githooks/pre-push` chama `scripts/ci.ps1 -Fast` (lint + typecheck + testes unitários rápidos). O hook é ativado por `pnpm prepare` (`git config core.hooksPath .githooks`).

**Testes:**
- **Protocolo:** vetores em `docs/protocol-vectors.json` rodados em **Rust, TypeScript e Kotlin**; leitura fragmentada, `magic` inválido, payload gigante, tipos desconhecidos, versão incompatível.
- **Rust:** unitários (`session`, `trust`, `compose`, `audio`) e integração com o `mock-sender`.
- **Frontends:** Vitest + React Testing Library; ESLint com `no-literal-string` (i18n).
- **Kotlin:** unitários do `ProtocolCodec`, `SendQueue`, `StateReducer`, `PairingStore`.
- **Matriz manual de aceitação (câmera):** Câmera do Windows, Chrome/Meet, OBS, Zoom, Teams, Discord, **NVIDIA Broadcast**. **(Microfone):** Configurações do Windows, Audacity, Discord, OBS.
- **Medições recorrentes:** latência ponta a ponta (câmera apontada para um cronômetro na tela do PC), fps real, CPU/GPU no PC, temperatura e bateria no celular após 30 min.

---

## 9. Riscos e mitigação

| Risco | Impacto | Mitigação |
|---|---|---|
| NVIDIA Broadcast não listar a câmera virtual | Objetivo principal falha | **S1** primeiro; plano B com filtro DirectShow |
| Permissões Frame Server ↔ app | Câmera virtual sem frames | **S2**; named pipe com DACL; testar sem elevar |
| Câmera dupla limitada ou ausente | Recurso reduzido | **S3**; detectar e esconder; rebaixar resolução; listar aparelhos testados |
| CameraX insuficiente (fps, simultâneas) | Retrabalho | Interface `CameraBackend`; S3 decide; troca localizada |
| WebCodecs/WebView2 inadequado para o preview | Preview ruim ou CPU alta | **S5**; plano B: preview reduzido gerado pelo Rust |
| Driver de áudio (kernel) | Bloqueios, tela azul, test signing | S4 só na Fase 8; reaproveitar driver existente; VM de teste; fase interina WASAPI |
| **Secure Boot desligado / modo de teste / HVCI** | Segurança reduzida; Memory Integrity pode bloquear o driver; **jogos com anti-cheat em modo kernel podem recusar rodar** | Documentar; avaliar driver de terceiros já assinado; separar entrada de boot de teste; checar os jogos que você usa |
| mDNS bloqueado (isolamento de clientes no Wi-Fi, firewall) | Reconexão automática falha | Cadeia de fallback: mDNS → últimos IPs → QR/manual |
| Encoders variam por fabricante | Artefatos em alguns aparelhos | Logar encoder; fallback de perfil; testar em ≥ 2 aparelhos |
| Rede sem criptografia | Tráfego legível na LAN | Risco aceito (uso próprio); aprovação + chave por dispositivo; TLS no backlog |
| APK e desktop em versões incompatíveis | Falhas confusas | Negociação de versão no `HELLO`; mensagem clara de "atualize" |
| Android encerrar o serviço em segundo plano | Transmissão cai | Foreground service; ajuda de bateria; reconexão automática |
| Decodificação duplicada (preview + câmera virtual) | CPU/GPU extra | Decidida pelo S5; opção de pausar preview |

---

## 10. Suposições a confirmar

1. Só **Android**; iPhone fora de escopo.
2. Sem prévia da câmera no celular no MVP (só status).
3. Um celular por vez.
4. Áudio mono PCM sem compressão no MVP.
5. Versões exatas (Capacitor 8.5, Node, JDK, Tauri, CLI do shadcn/ui para Tailwind 3.4) a confirmar na documentação antes de instalar.
6. O ADB não é embutido; é localizado ou baixado do Google com consentimento do usuário.
7. A chave por dispositivo e o token trafegam sem criptografia na LAN (consequência da decisão de segurança).

---

## 11. Backlog pós-v1.0

- Prévia da câmera no celular (fan-out de superfícies via CameraX effects/OpenGL).
- Seleção de lente (grande angular/tele), zoom, lanterna, foco/exposição remotos.
- TLS/Noise no canal; expiração do token de pareamento.
- Opus no áudio; HEVC.
- Efeitos no PC (desfoque/remoção de fundo, filtros).
- Instalação do APK via ADB pelo desktop.
- Foto e gravação local.
- Windows 10 (filtro DirectShow), Linux e macOS.
