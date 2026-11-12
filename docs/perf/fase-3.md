# Desempenho — Fase 3 (F3-T01)

Medições em `D:\Repositorios\MyLocalCamera` (Windows 11, RTX 3090,
2 monitores 1920×1080 @ 96 DPI), executadas em 2026-11-12.

| Item | Descrição | Meta | Medido | Resultado |
|---|---|---|---|---|
| PT-3.08 | Partida a frio do desktop: `Start-Process` do exe → probe UIA até a janela visível e até o conteúdo renderizado (wordmark "Local" na árvore UIA) | ≤ 2 s até utilizável | janela visível **0,70–0,81 s**; conteúdo renderizado **1,27–1,46 s** (3 rodadas com perfil WebView2 limpo em cada uma) | ✅ |

Metodologia (PT-3.08):

- 3 rodadas; antes de cada uma, perfil `%LOCALAPPDATA%\com.localcamera.desktop`
  removido (frio de verdade: criação de perfil WebView2 incluída).
- Exe debug do workspace (`target\debug\desktop.exe`) + dev server Vite
  (`localhost:1420`) já em execução — requisito de build debug do Tauri.
- Probe: `System.Windows.Automation` a cada 25 ms; "janela visível" =
  janela `LocalCamera` presente e `IsOffscreen=false`; "conteúdo" =
  texto "Local" visível no nó do conteúdo web
  (`Chrome_RenderWidgetHostHWND`).
- Observações: medido em máquina de dev sob carga variável (dev server,
  editor); números são referências de baseline — o restante da fase de
  performance é F3-T17.
