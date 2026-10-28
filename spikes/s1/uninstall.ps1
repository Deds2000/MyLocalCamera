#requires -Version 5.1
<#
.SYNOPSIS
  Removes the S1 spike: deletes the virtual camera, unregisters the DLL and
  deletes C:\Program Files\LocalCameraSpike.
.DESCRIPTION
  Camera removal mirrors the Microsoft sample's own uninstall path: it reads
  the camera metadata from the device interface registry keys and re-creates
  the IMFVirtualCamera via MFCreateVirtualCamera (P/Invoke) to call Remove()
  and Shutdown(). Requires administrator.
.NOTES
  Run from an ELEVATED (administrator) PowerShell window:
      powershell -ExecutionPolicy Bypass -File .\uninstall.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

function Test-Admin {
    $id   = [Security.Principal.WindowsIdentity]::GetCurrent()
    $pr  = New-Object Security.Principal.WindowsPrincipal($id)
    return $pr.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Admin)) {
    Write-Host ''
    Write-Host '[ERROR] This script requires administrator rights.' -ForegroundColor Red
    Write-Host 'Right-click PowerShell -> "Run as administrator", then re-run .\uninstall.ps1' -ForegroundColor Red
    exit 1
}

$DestDir = 'C:\Program Files\LocalCameraSpike'
$DestDll = Join-Path $DestDir 'VirtualCameraMediaSource.dll'
$ClSid   = '{7B89B92E-FE71-42D0-8A41-E137D06EA184}'

# ---------------------------------------------------------------------------
# 1) Remove the virtual camera (P/Invoke MFCreateVirtualCamera + Remove)
# ---------------------------------------------------------------------------
$cam = $null
$ifKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\DeviceInterfaces\{e5323777-f976-4f5b-9b55-b94699c46e44}'
if (Test-Path -LiteralPath $ifKey) {
    Get-ChildItem -LiteralPath $ifKey -ErrorAction SilentlyContinue | ForEach-Object {
        $p = Get-ItemProperty -LiteralPath $_.PSPath
        if ($p.'DeviceInterface_VCamCreate_SourceId' -eq $ClSid) { $cam = $p }
    }
}

if ($cam) {
    $friendlyName = [string]$cam.'DeviceInterface_VCamCreate_FriendlyName'
    $lifetime = 1   # MFVirtualCameraLifetime_System (default fallback)
    $access   = 0   # MFVirtualCameraAccess_CurrentUser (default fallback)
    if ($null -ne $cam.'DeviceInterface_VCamCreate_Lifetime') { $lifetime = [int]$cam.'DeviceInterface_VCamCreate_Lifetime' }
    if ($null -ne $cam.'DeviceInterface_VCamCreate_Access')   { $access   = [int]$cam.'DeviceInterface_VCamCreate_Access'   }

    Write-Host "Removing virtual camera '$friendlyName' (lifetime=$lifetime access=$access) ..."

    # C# compiled in-memory (PowerShell 5.1 / .NET Framework 4.x).
    # IMFAttributes + IMFVirtualCamera are declared with the exact vtable
    # order from the Windows SDK headers (mfobjects.h, mfvirtualcamera.h);
    # only Remove() and Shutdown() are ever called.
    $csharp = @'
using System;
using System.Runtime.InteropServices;

public static class VCamPinv
{
    [ComImport, Guid("6C088E2A-0CDB-4C3E-AF5C-0B9FD4D8ED28"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IMFAttributes
    {
        void GetItem(Guid guidKey, IntPtr pValue);
        void GetItemType(Guid guidKey, out int pType);
        void CompareItem(Guid guidKey, IntPtr Value, out int pbResult);
        void Compare(IntPtr pTheirs, int MatchType, out int pbResult);
        void GetUINT32(Guid guidKey, out uint punValue);
        void GetUINT64(Guid guidKey, out ulong punValue);
        void GetDouble(Guid guidKey, out double pfValue);
        void GetGUID(Guid guidKey, out Guid pguidValue);
        void GetStringLength(Guid guidKey, out uint pcchLength);
        void GetString(Guid guidKey, IntPtr pwszValue, uint cchBufSize, out uint pcchLength);
        void GetAllocatedString(Guid guidKey, out IntPtr ppwszValue, out uint pcchLength);
        void GetBlobSize(Guid guidKey, out uint pcbBlobSize);
        void GetBlob(Guid guidKey, IntPtr pBuf, uint cbBufSize, out uint pcbBlobSize);
        void GetAllocatedBlob(Guid guidKey, out IntPtr ppBuf, out uint pcbSize);
        void GetUnknown(Guid guidKey, ref Guid riid, out IntPtr ppv);
        void SetItem(Guid guidKey, IntPtr Value);
        void DeleteItem(Guid guidKey);
        void DeleteAllItems();
        void SetUINT32(Guid guidKey, uint unValue);
        void SetUINT64(Guid guidKey, ulong unValue);
        void SetDouble(Guid guidKey, double fValue);
        void SetGUID(Guid guidKey, Guid guidValue);
        void SetString(Guid guidKey, [MarshalAs(UnmanagedType.LPWStr)] string wszValue);
        void SetBlob(Guid guidKey, IntPtr pBuf, uint cbBufSize);
        void SetUnknown(Guid guidKey, IntPtr pUnknown);
        void LockStore();
        void UnlockStore();
        void GetCount(out uint pcItems);
        void GetItemByIndex(uint unIndex, out Guid pguidKey, IntPtr pValue);
        void CopyAllItems(IntPtr pDest);
    }

    [ComImport, Guid("1C08A864-EF6C-4C75-AF59-5F2D68DA9563"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IMFVirtualCamera : IMFAttributes
    {
        void AddDeviceSourceInfo([MarshalAs(UnmanagedType.LPWStr)] string DeviceSourceInfo);
        void AddProperty(IntPtr pKey, uint Type, IntPtr pbData, uint cbData);
        void AddRegistryEntry([MarshalAs(UnmanagedType.LPWStr)] string EntryName, [MarshalAs(UnmanagedType.LPWStr)] string SubkeyPath, uint dwRegType, IntPtr pbData, uint cbData);
        void Start(IntPtr pCallback);
        void Stop();
        void Remove();
        void GetMediaSource(out IntPtr ppMediaSource);
        void SendCameraProperty(Guid propertySet, uint propertyId, uint propertyFlags, IntPtr propertyPayload, uint propertyPayloadLength, IntPtr data, uint dataLength, out uint dataWritten);
        void CreateSyncEvent(Guid kseventSet, uint kseventId, uint kseventFlags, IntPtr eventHandle, out IntPtr cameraSyncObject);
        void CreateSyncSemaphore(Guid kseventSet, uint kseventId, uint kseventFlags, IntPtr semaphoreHandle, int semaphoreAdjustment, out IntPtr cameraSyncObject);
        void Shutdown();
    }

    // MF_VERSION = 0x00020713 (um\mfapi.h)
    [DllImport("mfplat.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    public static extern int MFStartup(uint version, uint flags);

    [DllImport("mfreadwrite.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    public static extern int MFCreateVirtualCamera(
        uint type, uint lifetime, uint access,
        [MarshalAs(UnmanagedType.LPWStr)] string friendlyName,
        [MarshalAs(UnmanagedType.LPWStr)] string mediaSource,
        IntPtr categories, uint categoryCount,
        out IntPtr ppVirtualCamera);

    public static int RemoveVirtualCamera(string friendlyName, string mediaSource, int lifetime, int access)
    {
        int hr = MFStartup(0x00020713, 0);
        if (hr < 0) { return hr; }

        IntPtr pUnk = IntPtr.Zero;
        hr = MFCreateVirtualCamera(0u, (uint)lifetime, (uint)access, friendlyName, mediaSource, IntPtr.Zero, 0u, out pUnk);
        if (hr < 0) { return hr; }

        try
        {
            IMFVirtualCamera vcam = (IMFVirtualCamera)Marshal.GetObjectForIUnknown(pUnk);
            vcam.Remove();
            vcam.Shutdown();
            return 0;
        }
        finally
        {
            Marshal.Release(pUnk);
        }
    }
}
'@
    Add-Type -TypeDefinition $csharp

    $hr  = 0
    $err = $null
    try {
        $hr = [VCamPinv]::RemoveVirtualCamera($friendlyName, $ClSid, $lifetime, $access)
    } catch {
        $err = $_.Exception.Message
        $hr  = -1
    }
    if ($err) {
        Write-Host "[ERROR] Failed to remove the virtual camera: $err" -ForegroundColor Red
        Write-Host 'Close any app using the camera (e.g. Camera app, Broadcast) and re-run.' -ForegroundColor Red
        Write-Host 'Alternatively remove it interactively: "C:\Program Files\LocalCameraSpike\VirtualCamera_Installer.exe" -> 2 -> select the camera.' -ForegroundColor Yellow
    } elseif ($hr -lt 0) {
        $hex = '{0:X8}' -f [uint32]$hr
        Write-Host "[ERROR] MFCreateVirtualCamera/Remove failed (HRESULT 0x$hex)." -ForegroundColor Red
        Write-Host 'Continuing with DLL unregistration and folder removal.' -ForegroundColor Yellow
    } else {
        Write-Host "Virtual camera '$friendlyName' removed." -ForegroundColor Green
    }
} else {
    Write-Host "No virtual camera with source $ClSid found (already removed?). Skipping camera removal." -ForegroundColor Yellow
}

# ---------------------------------------------------------------------------
# 2) Unregister the media source (remove the registry key — espelho do install)
#    (o sample NÃO usa regsvr32; registra via HKLM\...\CLSID\{GUID}\InprocServer32)
# ---------------------------------------------------------------------------
$ClsidKey = "HKLM:\SOFTWARE\Classes\CLSID\$ClSid"
if (Test-Path -LiteralPath $ClsidKey) {
    Write-Host "Removing registry key $ClsidKey ..."
    Remove-Item -LiteralPath $ClsidKey -Recurse -Force
    Write-Host "Registry key removed." -ForegroundColor Green
} else {
    Write-Host "Nenhuma chave CLSID $ClSid no registro (já removida?)." -ForegroundColor Yellow
}

# ---------------------------------------------------------------------------
# 3) Delete the folder
# ---------------------------------------------------------------------------
if (Test-Path -LiteralPath $DestDir) {
    Remove-Item -LiteralPath $DestDir -Recurse -Force
    Write-Host "Removed $DestDir." -ForegroundColor Green
}

Write-Host 'S1 spike uninstalled.' -ForegroundColor Green
