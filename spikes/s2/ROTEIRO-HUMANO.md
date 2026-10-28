# Roteiro — F0-S2 (parte do humano)

Spikes prontos pelo agente:

- **pipe-server** (o "app"): `spikes\s2\pipe-server\target\release\s2-pipe-server.exe`
- **shm-server**: `spikes\s2\shm-server\target\release\s2-shm-server.exe`
- **DLL adaptada** (1080p, cliente pipe/shm com fallback):
  `spikes\s2\dll\VirtualCameraMediaSource\x64\Release\VirtualCameraMediaSource.dll`
- **medidor de CPU**: `spikes\s2\measure-cpu.ps1`

Resultado esperado nos apps de câmera:
- **fundo VERDE + barra branca varrendo** = quadro vindo do servidor (canal OK);
- **gradiente azul/cinza rolante** = fallback da DLL (canal sem quadro).

Registre tudo nas tabelas de `docs/spikes/S2.md` (linha por linha) e devolva ao agente.

---

## 1. Preparo (PowerShell ELEVADA, uma vez)

```powershell
# 1.1) Copia DLL do spike S2 + installer do S1 para C:\Program Files\LocalCameraSpike
$src  = 'D:\Repositorios\MyLocalCamera\spikes\s2\dll\VirtualCameraMediaSource\x64\Release\VirtualCameraMediaSource.dll'
$inst = 'D:\Repositorios\MyLocalCamera\spikes\s1\_vendor\Windows-Camera\Samples\VirtualCamera\x64\Release\VirtualCamera_Installer.exe'
New-Item -ItemType Directory -Force 'C:\Program Files\LocalCameraSpike' | Out-Null
Copy-Item $src  'C:\Program Files\LocalCameraSpike\VirtualCameraMediaSource.dll'   -Force
Copy-Item $inst 'C:\Program Files\LocalCameraSpike\VirtualCamera_Installer.exe'    -Force

# 1.2) Registra a media source (mesmo fluxo do S1; NÃO usa regsvr32)
$clsid = '{7B89B92E-FE71-42D0-8A41-E137D06EA184}'
New-Item "HKLM:\SOFTWARE\Classes\CLSID\$clsid" -Force | Out-Null
New-Item "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Force | Out-Null
Set-ItemProperty "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name '(default)' -Value 'C:\Program Files\LocalCameraSpike\VirtualCameraMediaSource.dll'
Set-ItemProperty "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name 'Threading' -Value 'Both'

# 1.3) Cria a câmera virtual (mantenha a janela elevada)
& "C:\Program Files\LocalCameraSpike\VirtualCamera_Installer.exe"
```

Na tela do installer, responder:

```
1  -> register
1  -> VCam-SimpleMediaSource (synthetic test pattern)
1  -> lifetime: System
1  -> access: Current User
5  -> quit
```

A câmera **SWVCamMediaSource** deve aparecer em Configurações → Bluetooth e dispositivos → Câmeras.
(Se já existir uma "SWVCamMediaSource" antiga, remova antes com o installer → opção 2.)

## 2. Pipe — teste principal (usuário COMUM, nada elevado)

```powershell
# 2.1) Sobe o "app" (servidor de pipe) como usuário comum
cd D:\Repositorios\MyLocalCamera\spikes\s2\pipe-server
.\target\release\s2-pipe-server.exe --duration 3600 --out ..\results\s2-pipe-humano.json
```

O console avisa quando o cliente conecta/desconecta — **isso é a linha "app percebe
cliente conectado/desconectado"** da tabela.

1. Abra a câmera **SWVCamMediaSource** num app consumidor (app Câmera, Zoom, OBS…).
   - Anote o que aparece (verde+barra = canal OK; gradiente azul = fallback).
   - Preencha: *"DLL (Frame Server) abre o pipe"*.
2. **Matar o app com a câmera aberta**: feche o app consumidor → o servidor imprime
   desconexão; a câmera (se reaberta) volta em fallback azul. Reabra → reconecta.
   Preencha a linha correspondente.
3. **Abrir/fechar a câmera 20×**: feche e reabra o app consumidor 20 vezes.
   Anote se houve falha de reconexão (o servidor deve logar cada conexão).
4. **Reiniciar o Frame Server com o app ativo** (só este comando pede elevação):
   ```powershell
   Restart-Service FrameServer
   ```
   A câmera cai para fallback; ao reiniciar, a DLL reconecta. Anote.
5. **CPU (60 s)** com a câmera aberta, em OUTRO PowerShell (usuário comum):
   ```powershell
   cd D:\Repositorios\MyLocalCamera\spikes\s2
   powershell -ExecutionPolicy Bypass -File .\measure-cpu.ps1
   ```
6. **Vazão/latência**: o servidor termina sozinho (ou Ctrl+C) e imprime o resumo
   (quadros, MB/s, latência p50/p95) e grava `results\s2-pipe-humano.json`.
   Copie para a tabela.

## 3. Segurança — outro usuário e outro PC

> O servidor do pipe precisa estar rodando (passo 2.1).

1. **Outro usuário local** (sessão nova desse usuário, PowerShell comum):
   ```powershell
   try { [System.IO.File]::Open('\\\\.\\pipe\\LocalCameraSpike','Open','ReadWrite').Dispose(); 'ABERTO' }
   catch { 'NEGADO: ' + $_.Exception.Message }
   ```
   Esperado: **NEGADO** (DACL só libera o usuário que criou + LocalService).
2. **Cliente remoto** (de outro PC na rede; `NOME_DO_PC` = nome da máquina principal):
   ```powershell
   try { [System.IO.File]::Open('\\NOME_DO_PC\pipe\LocalCameraSpike','Open','ReadWrite').Dispose(); 'ABERTO' }
   catch { 'NEGADO: ' + $_.Exception.Message }
   ```
   Esperado: **NEGADO** (`PIPE_REJECT_REMOTE_CLIENTS`). Sem segundo PC: anote "n/a".

## 4. Memória compartilhada Global\ (usuário COMUM)

1. **Criar o objeto sem elevação**:
   ```powershell
   cd D:\Repositorios\MyLocalCamera\spikes\s2\shm-server
   .\target\release\s2-shm-server.exe --duration 30 --out ..\results\s2-shm-humano.json
   ```
   Esperado: `CreateFileMappingW` → **ACCESS_DENIED**, JSON com `"created": false`
   (exit 2). Copie o erro exato para a tabela.
2. **DLL em modo shm** (elevação só para gravar HKLM):
   ```powershell
   New-Item 'HKLM:\SOFTWARE\LocalCameraSpike' -Force | Out-Null
   New-ItemProperty 'HKLM:\SOFTWARE\LocalCameraSpike' -Name Channel -Value shm -PropertyType String
   ```
   Feche e reabra a câmera no app consumidor. A DLL tenta `Global\LocalCameraSpike`
   (que o app NÃO conseguiu criar) → esperado: **fallback azul** continuamente.
   Preencha *"DLL abre/lê"*.
3. Para voltar ao pipe:
   ```powershell
   Remove-ItemProperty 'HKLM:\SOFTWARE\LocalCameraSpike' -Name Channel
   ```
   e feche/reabra a câmera.
4. **(mesmas medições)**: n/a — não há objeto criado (criação negada). O mecanismo
   mapping+evento já foi validado em `Local\` pelo smoke-client do agente
   (`results\s2-shm-local.json`: 92 quadros, 0 perdas, 95,2 MB/s).

## Apêndice — recompile a DLL (só se o binário não existir)

```powershell
& 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe' `
  'D:\Repositorios\MyLocalCamera\spikes\s2\dll\VirtualCameraMediaSource\VirtualCameraMediaSource.vcxproj' `
  /t:Rebuild /p:Configuration=Release /p:Platform=x64 /p:WindowsTargetPlatformVersion=10.0.22621.0 `
  '/p:SolutionDir=D:\Repositorios\MyLocalCamera\spikes\s2\dll\'
```

Saída: `spikes\s2\dll\VirtualCameraMediaSource\x64\Release\VirtualCameraMediaSource.dll`
(0 warnings; pacotes CppWinRT/WIL em `spikes\s2\dll\packages\`).

## 5. Limpeza (opcional, PowerShell elevada)

```powershell
cd D:\Repositorios\MyLocalCamera\spikes\s1
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1   # remove câmera, CLSID e C:\Program Files\LocalCameraSpike
Remove-Item 'HKLM:\SOFTWARE\LocalCameraSpike' -Force -ErrorAction SilentlyContinue
```

---

## Observações para o preenchimento das tabelas

- Se qualquer linha não foi executada, escreva `n/a` + motivo.
- Anote a **hora** de cada bloco (útil para correlacionar com os logs).
- O JSON do pipe-server traz `frames_written`, `throughput_mbps`,
  `latency_us_p50/p95/max` — são os números da tabela.
- **Não** rode o pipe-server elevado ("como administrador" invalida o teste).
