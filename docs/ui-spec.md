# LocalCamera — Especificação de UI (índice, design system e regras compartilhadas)

Vale para `desktop/` e `mobile/`. Stack e decisões em `LocalCamera-PLAN.md`.

| Documento | Conteúdo |
|---|---|
| **`docs/ui-spec.md`** (este) | Princípios, design system (tokens, glass, tipografia, ícones, movimento, componentes, estados), marca, desempenho, i18n, acessibilidade, checklist de revisão |
| `docs/ui/desktop.md` | **§2** — janela, wireframes, preview, painel lateral campo a campo, rodapé de saúde, diálogos, assistente, atalhos, estados vazios/erro, árvore de componentes e stores |
| `docs/ui/mobile.md` | **§3** — navegação, cada tela com wireframe, estados, erros, Android (tema, ícone, notificação), componentes |
| `docs/ui/copy.md` | Todos os textos iniciais em pt-BR e en, com as chaves de i18n |

> **§2 = `docs/ui/desktop.md`** e **§3 = `docs/ui/mobile.md`**. As fases citam "ui-spec §2/§3" com esse significado.

---

## 1. Design system

### 1.1 Princípios
1. **O estado da conexão nunca é ambíguo.** Toda tela mostra, sem rolar, se está desconectado, aguardando, transmitindo ou reconectando.
2. **Cor nunca é o único sinal.** Todo estado combina cor + ícone + texto.
3. **Padrão seguro.** Em diálogos de risco (aprovar dispositivo, revogar, parar), o foco inicial fica na opção segura.
4. **Vidro com moderação.** Glassmorphism dá identidade, mas custa GPU e bateria: no máximo **2 camadas de blur sobrepostas** e **nunca blur sobre o vídeo**.
5. **O vídeo é o protagonista.** Controles ficam em volta e somem quando não são necessários (tela cheia).
6. **Rápido primeiro.** Nada de animação ou renderização por quadro; veja §4.

### 1.2 Tokens de cor
Tema **somente escuro**. Valores no preset `packages/shared/tailwind-preset.ts` e nas variáveis CSS do shadcn (HSL, sem `hsl()` — formato `H S% L%` do shadcn). HSL aproximado (≈); ajustar se a conversão exata diferir.

| Papel | Token Tailwind | Hex | HSL (≈) |
|---|---|---|---|
| Fundo | `bg` | `#070B14` | `221 49% 5%` |
| Fundo elevado | `bg-raised` | `#0C1322` | `221 48% 9%` |
| Texto | `text` | `#E6EDF7` | `215 52% 94%` |
| Texto secundário | `text-muted` | `#8FA1BD` | `217 26% 65%` |
| Destaque | `accent` | `#22D3EE` | `188 86% 53%` |
| Destaque forte (hover/pressionado) | `accent-strong` | `#06B6D4` | `189 94% 43%` |
| Destaque azul (fim do gradiente) | `accent-blue` | `#3B82F6` | `217 91% 60%` |
| Sucesso | `ok` | `#34D399` | `158 64% 52%` |
| Aviso | `warn` | `#FBBF24` | `43 96% 56%` |
| Perigo | `danger` | `#FB7185` | `351 95% 71%` |
| Vidro (superfície) | `glass` | branco 5% | — |
| Vidro forte | `glass-strong` | branco 9% | — |
| Borda de vidro | `glass-border` | branco 10% | — |

**Mapeamento para as variáveis do shadcn/ui:**

| Variável | Valor |
|---|---|
| `--background` | `221 49% 5%` |
| `--foreground` | `215 52% 94%` |
| `--card` / `--popover` | `221 48% 9%` |
| `--card-foreground` / `--popover-foreground` | `215 52% 94%` |
| `--primary` | `188 86% 53%` |
| `--primary-foreground` | `221 49% 5%` (texto **escuro** sobre o ciano) |
| `--secondary` | `221 30% 14%` |
| `--secondary-foreground` | `215 52% 94%` |
| `--muted` | `221 30% 14%` |
| `--muted-foreground` | `217 26% 65%` |
| `--accent` | `221 30% 18%` (hover de itens; **não** é a cor de destaque) |
| `--accent-foreground` | `215 52% 94%` |
| `--destructive` | `350 89% 60%` |
| `--destructive-foreground` | `215 52% 94%` |
| `--border` / `--input` | `225 16% 15%` |
| `--ring` | `188 86% 53%` |
| `--radius` | `0.75rem` |

Contraste mínimo AA (4.5:1 texto normal, 3:1 texto grande e componentes). **Verificar** `text-muted` sobre `bg-raised` e sobre vidro; se ficar abaixo, clarear o token.

### 1.3 Superfícies (glass)

| Nível | Uso | Receita Tailwind |
|---|---|---|
| `surface-0` | Fundo da janela | `bg-bg` + gradiente (abaixo) |
| `surface-1` | Cartões, painel lateral | `bg-glass backdrop-blur-xl border border-glass-border shadow-glass` |
| `surface-2` | Itens elevados dentro de cartões (linhas, chips) | `bg-glass-strong border border-glass-border` (sem blur) |
| `overlay` | Diálogos, menus, sheets | `bg-bg-raised/85 backdrop-blur-2xl border border-glass-border shadow-overlay` |

- **Gradiente de fundo:** `linear-gradient(160deg, #070B14 0%, #0A1224 55%, #08101F 100%)` + dois *glows* radiais de `accent` a 8–12% (canto superior esquerdo) e `accent-blue` a 8% (canto inferior direito).
- **Sombras:** `shadow-glass` = `0 8px 32px rgb(0 0 0 / .35), inset 0 1px 0 rgb(255 255 255 / .06)`; `shadow-overlay` = `0 24px 64px rgb(0 0 0 / .55), inset 0 1px 0 rgb(255 255 255 / .08)`; `glow-accent` = `0 0 24px rgb(34 211 238 / .25)`.
- **Regras de blur:** (a) `surface-2` nunca usa blur; (b) não aninhar mais de 2 níveis com blur; (c) o preview e a área de vídeo **não** recebem `backdrop-filter` por cima; (d) se o desempenho cair (mobile fraco, GPU integrada), a classe `reduce-glass` no `<html>` troca blur por fundo sólido `bg-bg-raised`. No mobile, ela é **ligada automaticamente durante a transmissão**.
- Efeito de janela do Windows (Mica/Acrylic via crate de vibrancy): **opcional**, fora do MVP; confirmar suporte antes.

### 1.4 Tipografia
Fontes embutidas (`@fontsource-variable/inter`, `@fontsource-variable/jetbrains-mono`); fallback `system-ui, sans-serif` e `ui-monospace, monospace`.

| Estilo | Desktop | Mobile | Peso | Uso |
|---|---|---|---|---|
| `display` | 28/36 | 28/36 | 600 | Título do estado vazio, estado grande no celular |
| `title` | 20/28 | 20/28 | 600 | Títulos de diálogo/tela |
| `section` | 12/16, MAIÚSC., `tracking-[0.08em]` | 12/16 | 600 | Rótulos de seção do painel |
| `body` | 14/20 | **16/24** | 400 | Texto geral |
| `body-strong` | 14/20 | 16/24 | 500 | Rótulos de controles |
| `small` | 12/16 | 13/18 | 400 | Ajuda, legendas |
| `stat` | 13/16 mono | 14/18 mono | 500 | Números (fps, ms, Mbps), com `font-variant-numeric: tabular-nums` |

### 1.5 Espaçamento, raios e dimensões
- Base de **4 px**; escala `1·2·3·4·5·6·8·10·12` (4–48 px).
- Raios: `sm` 8, `md` 12 (controles), `lg` 16 (cartões), `xl` 24 (moldura do preview e sheets), `full` (chips, botões redondos).
- Altura dos controles — desktop: 32 (compacto), 36 (padrão), 44 (grande). Mobile: **mínimo 44**, botões principais 52–56.
- Ícones: 16 (inline), 20 (botões), 24 (destaque); traço 1.75; sempre `aria-hidden` quando há texto ao lado.

### 1.6 Ícones (`lucide-react`)
**Confirmar os nomes na versão instalada** (alguns ícones foram renomeados entre versões).

| Significado | Ícone | Significado | Ícone |
|---|---|---|---|
| Câmera / sem câmera | `Camera` / `CameraOff` | Trocar câmera | `SwitchCamera` |
| Vídeo / sem vídeo | `Video` / `VideoOff` | Espelhar | `FlipHorizontal2` |
| Microfone / mudo | `Mic` / `MicOff` | Girar | `RotateCw` |
| Wi-Fi / sem Wi-Fi | `Wifi` / `WifiOff` | PiP | `PictureInPicture2` |
| USB | `Usb` | Tela cheia / sair | `Maximize` / `Minimize` |
| QR code / escanear | `QrCode` / `ScanLine` | Configurações | `Settings` |
| Celular / PC | `Smartphone` / `Monitor` | Idioma | `Languages` |
| Confiável / alerta | `ShieldCheck` / `ShieldAlert` | Copiar / mostrar / ocultar | `Copy` / `Eye` / `EyeOff` |
| Excluir | `Trash2` | Atualizar / carregando | `RefreshCw` / `Loader2` (girando) |
| Latência / qualidade | `Activity` / `Gauge` | Aviso / info / ok | `TriangleAlert` / `Info` / `Check` |
| Baixar | `Download` | Fechar / minimizar | `X` / `Minus` |

### 1.7 Movimento
- Durações: `fast` 120 ms, `base` 200 ms, `slow` 320 ms. Easing padrão `cubic-bezier(0.22, 1, 0.36, 1)`.
- **Desktop (framer-motion):** troca de seção do painel = fade + deslocamento de 8 px; cartões = entrada com fade + scale 0.98→1; diálogos = scale 0.96→1 + fade; mola dos painéis `{ type: 'spring', stiffness: 300, damping: 30 }`; contagem animada de números (300 ms) só nos chips e estatísticas.
- **Pulso do "AO VIVO":** ponto com anel que expande e esmaece em 1,6 s, infinito, **só `transform` e `opacity`**.
- **Mobile:** somente CSS (`transition`, `@keyframes`), sem framer-motion; animações pausam quando a tela está apagada ou o app em segundo plano.
- **`prefers-reduced-motion`:** remover deslocamentos/escalas e usar apenas fade de 0–80 ms; o ponto de "AO VIVO" fica estático.

### 1.8 Estados interativos (todos os controles)
| Estado | Regra |
|---|---|
| Hover (desktop) | Fundo +4% de branco; bordas +4% |
| Pressionado | `scale(0.98)` (só desktop; no mobile, escurecer 8%) |
| Foco por teclado | Anel de 2 px `accent` com offset de 2 px sobre `bg` — **sempre visível** |
| Desabilitado | Opacidade 45%, cursor `not-allowed`, **com tooltip explicando o motivo** (desktop) ou texto de ajuda (mobile) |
| Carregando | Spinner no lugar do ícone, botão desabilitado, largura preservada |
| Erro de campo | Borda `danger`, ícone `TriangleAlert`, mensagem abaixo (`aria-describedby`) |

### 1.9 Semântica de estado de conexão
Usada em chips, pílula da titlebar, rodapé, tela do celular e notificação.

| Estado | Cor | Ícone | Texto (chave) | Animação |
|---|---|---|---|---|
| Desconectado (`idle`) | `text-muted` | `WifiOff` | `common.state.idle` | — |
| Conectando | `accent` | `Loader2` | `common.state.connecting` | giro |
| Aguardando aprovação (`pending`) | `warn` | `ShieldAlert` | `common.state.pending` | respiração suave |
| Transmitindo (`streaming`) | `ok` | `Wifi`/`Usb` | `common.state.streaming` | pulso do ponto |
| Reconectando | `warn` | `RefreshCw` | `common.state.reconnecting` | giro |
| Erro / revogado | `danger` | `TriangleAlert` | código → `common.errors.*` | — |

### 1.10 Inventário de componentes

**shadcn/ui (instalar e adaptar ao tema):** Button, Badge, Tabs, Accordion, Select, Slider, Switch, ToggleGroup, RadioGroup, Input, Label, Tooltip, Dialog, AlertDialog, Sheet, DropdownMenu, ScrollArea, Separator, Progress, Skeleton, Sonner (toasts).

**Componentes próprios** (em `packages/shared/src/ui` quando usados nos dois apps e **sem** framer-motion; os animados ficam em `desktop/src`):

| Componente | Onde | Props principais | Notas |
|---|---|---|---|
| `StatusDot` | shared | `state`, `size` | Segue §1.9; o pulso é CSS |
| `StatusChip` | shared | `state`, `label`, `icon?` | Pílula de 24–28 px |
| `StatTile` | shared | `label`, `value`, `unit`, `hint?` | Número em `stat`, rótulo em `small` |
| `SegmentedControl` | shared | `options`, `value`, `onChange`, `disabledReasons?` | Base: `ToggleGroup`; opções desabilitadas com motivo |
| `SettingRow` | shared | `label`, `description?`, `control`, `appliesTo?` | `appliesTo: 'phone' \| 'pc'` mostra ícone sutil; `phone` = reinicia captura |
| `DeviceRow` | shared | `icon`, `title`, `subtitle`, `badge`, `action` | Linha de aparelho USB / confiável / PC conhecido |
| `EmptyState` | shared | `icon`, `title`, `description`, `actions` | |
| `LevelMeter` | shared | `level`, `peak` | Desenhado em canvas ou CSS com transform; ≤ 20 Hz |
| `HealthIndicator` | desktop | `kind`, `state`, `detail` | Tooltip + clique abre a correção |
| `PreviewFrame` | desktop | `children`, `overlay` | Moldura 16:9, raio `xl`, letterbox |
| `QRCard` | desktop | `uri`, `hosts`, `port` | QR com margem clara de 16 px (contraste para leitura) |
| `ApprovalDialog` | desktop | `request` | Ver `docs/ui/desktop.md` |
| `CountdownRing` | mobile | `seconds`, `total` | SVG com `stroke-dashoffset` (CSS) |
| `BigToggle` | mobile | `icon`, `label`, `pressed` | Botão 56 px com estado claro |

> **Catálogo de desenvolvimento:** cada app tem uma rota/página `?catalog` (apenas em *dev*) que mostra todos os componentes em todos os estados. Serve de revisão visual e evita regressões.

---

## 2. Desktop → ver `docs/ui/desktop.md`
## 3. Mobile → ver `docs/ui/mobile.md`

---

## 4. Regras de desempenho da UI (obrigatórias)

- O **backend agrega as estatísticas e emite `session://stats` a no máximo 2 Hz** (a cada 500 ms). Nada de evento por quadro.
- Os quadros do preview são desenhados **de forma imperativa** (`canvas` via `ref`), **fora do ciclo de renderização do React**. Nenhum `setState` por quadro.
- Estado global com `zustand` e **seletores estreitos**; cada componente assina só o pedaço de que precisa.
- Pausar o preview quando a janela está minimizada ou oculta (`visibilitychange`). A câmera virtual **continua** funcionando.
- Medidor de nível de áudio: no máximo 15–20 Hz.
- Listas e logs longos com limite (ex.: 500 linhas) e virtualização se necessário.
- Blur e sombras: conforme §1.3; `reduce-glass` disponível nos dois apps.
- Imagens/ícones do app embutidos como SVG; nada carregado de CDN.

## 5. Internacionalização

- `i18next` + `react-i18next`; recursos em `packages/shared/src/i18n/{pt-BR,en}.json`, namespaces `common`, `desktop`, `mobile`. **Os textos iniciais estão em `docs/ui/copy.md`** — copiar para os JSON na Fase 2/3.
- Idioma inicial detectado do sistema (`pt*` → pt-BR; qualquer outro → en); preferência do usuário "Sistema / Português / English"; troca em tempo real.
- ESLint `i18next/no-literal-string` ativo nos dois frontends: **nenhum texto fixo na UI**.
- Rust e Kotlin enviam **códigos** (de erro, de `reason`, de estado), nunca frases. Mapeamento código → chave em `common.errors.*`.
- Plurais e interpolação via i18next (`{{count}}`, `_one`/`_other`); números e unidades via `Intl.NumberFormat`.
- Textos nativos do Android (notificação, nome do canal, permissões) em `res/values/strings.xml` (en) e `res/values-pt-rBR/strings.xml`. Chaves listadas em `docs/ui/copy.md`.
- Nomes de dispositivos virtuais **não** são traduzidos ("LocalCamera", "LocalCamera Back", "LocalCamera Front", "LocalCamera Microphone").
- Convenção de chaves: `area.tela.elemento` (ex.: `desktop.connection.qrTitle`). Textos longos podem crescer ~30% em pt-BR: **nenhum layout pode depender do tamanho do texto em inglês**.

## 6. Acessibilidade

- **Teclado (desktop):** ordem de tabulação lógica; todos os controles alcançáveis; atalhos em `docs/ui/desktop.md` §2.9; `Esc` fecha diálogos e sai da tela cheia.
- **Leitores de tela:** rótulos (`aria-label`) em todo botão só com ícone; regiões (`role="region"`, `aria-label`) para preview, painel e rodapé; mudanças de estado de conexão anunciadas em `aria-live="polite"`; erros em `role="alert"`.
- **Alvos:** desktop ≥ 28 px; mobile ≥ 44 px.
- **Contraste AA**; foco sempre visível; nenhum estado só por cor.
- **Mobile:** respeitar escala de fonte do sistema (layouts em `rem`, sem alturas fixas em texto); suporte a TalkBack nos fluxos principais (Home → Scan → Transmitindo → Parar).
- **Movimento:** `prefers-reduced-motion` respeitado (§1.7).

## 7. Marca

- **Ícone do app:** quadrado arredondado com gradiente `accent` → `accent-blue`, uma **lente** (círculo com anel interno) e um pequeno arco de "sinal" no canto superior direito. SVG simples, **legível em 16 px**, sem texto. No Android, ícone adaptativo (primeiro plano = lente + arco; fundo = gradiente) e **ícone monocromático** para a notificação.
- **Logotipo:** ícone + "LocalCamera" em Inter 600; "Local" em `text`, "Camera" em `accent`.
- **Tela inicial (splash) Android:** fundo `bg`, ícone centralizado (API de splash do Android 12+; confirmar na documentação).
- Entregar fontes em SVG em `packages/shared/src/assets/` e PNG/ICO/ICNS gerados por script a partir do SVG.

## 8. Checklist de revisão de UI (por tela, antes de fechar a fase)

- [ ] Todos os estados existem no catálogo (vazio, carregando, normal, erro, desabilitado, longo texto em pt-BR).
- [ ] Nenhum texto fixo; pt-BR e en revisados; textos longos não quebram o layout.
- [ ] Cor + ícone + texto em todo estado; contraste AA verificado.
- [ ] Foco visível e ordem de tabulação correta; rótulos ARIA nos botões de ícone.
- [ ] Alvos de toque ≥ 44 px (mobile).
- [ ] Sem `backdrop-filter` sobre o vídeo; no máximo 2 camadas de blur.
- [ ] Nenhuma renderização por quadro (React Profiler); estatísticas ≤ 2 Hz.
- [ ] `prefers-reduced-motion` testado.
- [ ] Redimensionar a janela até o mínimo (desktop) e fonte do sistema em 130–200% (mobile) sem cortes.
- [ ] Capturas de tela da tela em `docs/ui/screens/` para comparação futura.
