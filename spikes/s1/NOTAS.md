# S1 — NOTAS do build (parte do agente)

Data: 2026-10-27 · Windows 11 · GPU RTX 3090 (driver NVIDIA 617.14) · NVIDIA Broadcast instalado
MSBuild 17.11.9 (VS BuildTools 2022) · Windows SDK 10.0.22621.0

## Origem do sample

- `git clone --depth 1 https://github.com/microsoft/Windows-Camera spikes/s1/_vendor/Windows-Camera`
- Commit clonado: `626f8b1` (2026-09-21)
- Caminho do sample (confirmado no README raiz, linha 22): `Samples/VirtualCamera`
- Solution: `Samples/VirtualCamera/VirtualCameraSample.sln`

Projetos do solution (não compila todos; só o necessário para o spike):

| Projeto | Tipo | Usado? |
|---|---|---|
| `VirtualCameraMediaSource` | DLL (media source da câmera virtual) | Sim |
| `VirtualCamera_Installer` | Exe console (registra/remove/testa câmera virtual) | Sim |
| `VirtualCameraManager_WinRT` | DLL WinRT | Não (sem config Release\|x64 no solution) |
| `VirtualCameraManager_App` | App UWP | Não (exige workload UWP ausente nos BuildTools) |
| `VirtualCameraSystray` | Exe C# (bandeja) | Não (opcional, só p/ AugmentedMediaSource) |
| `VirtualCamera_MSI` | vdproj MSI | Não (exige extensão MSI do VS) |
| `VirtualCameraTest` | Exe de testes | Não |

## Comandos usados (raiz do repositório)

1. Restauração de NuGet (o `msbuild /t:Restore` NÃO restaura projetos C++ com
   `packages.config`; é preciso o NuGet clássico):
   ```powershell
   & 'C:\Users\PC\AppData\Local\Temp\opencode\nuget.exe' restore 'spikes/s1/_vendor/Windows-Camera/Samples/VirtualCamera/VirtualCameraSample.sln'
   ```
   (nuget.exe baixado de `https://dist.nuget.org/win-x86-commandline/latest/nuget.exe`;
   pacotes caem em `Samples\VirtualCamera\packages\`: Microsoft.Windows.CppWinRT 3.0.260520.1,
   Microsoft.Windows.ImplementationLibrary 1.0.260126.7, Microsoft.VCRTForwarders.140 1.1.0, googletest 1.8.1.8)

2. Build (os projetos têm `WindowsTargetPlatformVersion=10.0.26100.0` hardcoded;
   sobrescrito para o SDK instalado via `/p:`). Build por projeto, não pelo solution:
   ```powershell
   $msb = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe'
   $base = 'D:\Repositorios\MyLocalCamera\spikes\s1\_vendor\Windows-Camera\Samples\VirtualCamera'
   & $msb "$base\VirtualCameraMediaSource\VirtualCameraMediaSource.vcxproj"  /t:Rebuild /p:Configuration=Release /p:Platform=x64 /p:WindowsTargetPlatformVersion=10.0.22621.0 "/p:SolutionDir=$base\"
   & $msb "$base\VirtualCamera_Installer\VirtualCamera_Installer.vcxproj" /t:Rebuild /p:Configuration=Release /p:Platform=x64 /p:WindowsTargetPlatformVersion=10.0.22621.0 "/p:SolutionDir=$base\"
   ```
   (`/p:SolutionDir` é preciso porque os imports de NuGet do projeto usam `$(SolutionDir)packages\...`.)

## Artefatos gerados

- DLL:   `spikes/s1/_vendor/Windows-Camera/Samples/VirtualCamera/x64/Release/VirtualCameraMediaSource.dll` (1.027.072 bytes)
- Exe:   `spikes/s1/_vendor/Windows-Camera/Samples/VirtualCamera/x64/Release/VirtualCamera_Installer.exe` (148.992 bytes)

Resultados: ambos com exit code 0. **Warnings: 0 nos dois projetos** (rebuild completo).
Obs.: rebuild do solution inteiro falha nos *outros* projetos (MSI vdproj não suportado sem a
extensão MSI; app UWP sem workload UWP; `VirtualCameraManager_WinRT`/`CameraKsPropertyHelper`
sem configuração Release|x64 mapeada) — irrelevante para o spike.

## Executáveis e como criar a câmera

`VirtualCamera_Installer.exe` é um console interativo (rode elevado):

```
1 - register | 2 - remove | 3 - TestVCam | 4 - TestCustomControl | 5 - quit
```

- **1 (register)** → `1 - VCam-SimpleMediaSource` (síntese, sem câmera física; o
  "padrão de teste" do sample: quadriculado + cores via KSProperty custom) →
  lifetime `1 = System` (ou `2 = Session`) → acesso `1 = CurrentUser` (ou `2 = All`).
  Nome amigável registrado: **`SWVCamMediaSource`**.
- **2 (remove)** → lista as câmeras virtuais e remove a selecionada
  (o sample recria a `IMFVirtualCamera` e chama `Remove()` + `Shutdown()`).
- Opção 3 testa streaming direto do source (in-process).
- Modo não interativo: `VirtualCamera_Installer.exe /Uninstall` (só remove o MSI do sample,
  não aplica a instalação manual via `install.ps1`).

Host de criação: `install.ps1` copia DLL + exe para `C:\Program Files\LocalCameraSpike\`,
registra a DLL com `regsvr32 /s` e imprime o comando exato de criação.

## `MFCreateVirtualCamera` — parâmetros usados pelo sample

Fonte: `VirtualCameraTest/VCamUtils.cpp::RegisterVirtualCamera()` (linha 113) e
`VirtualCameraManager_WinRT/VirtualCameraRegistrar.cpp::CreateVirtualCamera()` (linha 39).

| Parâmetro | Valor |
|---|---|
| Type | `MFVirtualCameraType_SoftwareCameraSource` (0) |
| Lifetime | `MFVirtualCameraLifetime_System` (1) ou `MFVirtualCameraLifetime_Session` (0) — escolhido interativamente |
| Access | `MFVirtualCameraAccess_CurrentUser` (0) ou `MFVirtualCameraAccess_AllUsers` (1) — escolhido interativamente |
| friendlyName | `L"SWVCamMediaSource"` (Simple); nome da câmera física quando envolta (HW/Augmented) |
| mediaSource | `L"{7B89B92E-FE71-42D0-8A41-E137D06EA184}"` (`CLSID_VirtualCameraMediaSource`, `VirtualCameraMediaSource.h`) |
| categories | `nullptr`, count 0 |
| Pós-criação | `AddDeviceSourceInfo(symLink)` só quando envolve câmera física; `SetUINT32(VCAM_KIND, ...)` no caminho WinRT; `AddProperty(DEVPKEY_DeviceInterface_VCamCreate_SourceId, ...)` no `VCamUtils::RegisterVirtualCamera` |

Enum values confirmadas na header do SDK instalado
(`um\mfvirtualcamera.h`): Type 0; Lifetime Session=0/System=1; Access CurrentUser=0/AllUsers=1.

## Requisitos/observações para o humano

- A DLL roda dentro do **FrameServer** (outro usuário) por isso `C:\Program Files\LocalCameraSpike\`.
- O exe requer o **VC++ 2022 Redistributable** (README do sample: "Latest VCRuntime redistributable installed").
- O sample é Windows 11 22000+; aqui temos 22621 → OK.
- `uninstall.ps1` remove a câmera via P/Invoke de `MFCreateVirtualCamera` + `Remove()`
  (mesmo caminho do sample), lê os metadados da câmera no registro
  (`HKLM\SYSTEM\CurrentControlSet\Control\DeviceInterfaces\{e5323777-...}\*`),
  desregistra a DLL (`regsvr32 /s /u`) e apaga a pasta.
