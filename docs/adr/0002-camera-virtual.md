# ADR 0002 — Câmera virtual por Media Foundation (sem fallback DirectShow)
- **Data:** 2026-10-28
- **Status:** aceito
## Contexto
A Fase 4 precisa apresentar o vídeo do celular como webcam no Windows 11. Duas rotas eram candidatas:
câmera virtual **Media Foundation** (`MFCreateVirtualCamera` + media source custom) ou filtro
**DirectShow** (Plano B). O spike S1 testou a rota MF com a media source do sample Microsoft
(`SimpleMediaSource`, padrão de teste) neste PC (Win 11, RTX 3090, driver 617.14, NVIDIA Broadcast
instalado). Casos avaliados: listagem e entrega de frames em apps reais — o caso decisivo sendo o
**NVIDIA Broadcast**.

## Decisão
- **Media Foundation** (`MFCreateVirtualCamera` + media source própria do produto) é a rota da Fase 4.
  **Sem fallback DirectShow** — o Broadcast listou e recebeu a câmera MF.
- A DLL da media source é instalada em **caminho máquina** (`C:\Program Files\...`) — ela é carregada
  pelo **Frame Server** (outro usuário/sessão).
- O **registro do CLSID** é feito gravando o registro:
  `HKLM\SOFTWARE\Classes\CLSID\{GUID}\InprocServer32` com `(default)=caminho da DLL` e `Threading=Both`.
  **Não usar `regsvr32`** (a media source do sample não exporta `DllRegisterServer`; a nossa segue o modelo).
- Criar/remover a câmera via `MFCreateVirtualCamera` (P/Invoke — o `spikes/s1/uninstall.ps1` já
  implementa a remoção).
- Assinatura da DLL **não é pré-requisito técnico** (funcionou sem, em todos os apps testados,
  incluindo Broadcast) — decisão de distribuição fica para o empacotamento final.

## Consequências
- **Fase 4:** novo projeto C++ (`virtualcam/`) com a media source do produto (consome o feed do Frame
  Server), installer (copiar DLL + registrar CLSID + criar câmera) e remover câmera no desinstalar.
- **Validar na Fase 4:** uso simultâneo por 2+ apps (Meet + OBS), ciclo de vida do Broadcast
  (abrir/fechar/efeitos), e comportamento com o celular offline (media source não pode derrubar o
  app consumidor).
- **Spike S1 desinstalado** após o veredito (`spikes/s1/uninstall.ps1`) — a câmera de teste
  `SWVCamMediaSource` não fica no PC.

## Alternativas consideradas
- **Filtro DirectShow (Plano B):** descartado — a rota MF funcionou em todos os apps relevantes,
  incluindo o NVIDIA Broadcast, que era o motivo de ter o Plano B.
- **OBS VirtualCam / câmera virtual de terceiros como base:** não é API estável nem controlável —
  o produto precisa da própria media source.
