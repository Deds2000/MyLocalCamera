//
// Spike S2 — canal entre o app do usuário e esta DLL (que roda no Frame
// Server, conta LocalService, sessão 0).
//
// A DLL é o CLIENTE do canal; o app do usuário é o servidor que publica os
// quadros. O canal é escolhido em tempo de execução pelo registro
// HKLM\SOFTWARE\LocalCameraSpike\Channel:
//   "pipe" (padrão) : named pipe \\.\pipe\LocalCameraSpike
//   "shm"           : memória mapeada Global\LocalCameraSpike + evento
//                     Global\LocalCameraSpikeEvt
//
// Um thread de fundo lê o quadro mais recente (NV12 1920x1080) e o guarda em
// um buffer privado; TryFillFrame() faz o blit do quadro mais recente no
// buffer do sample e devolve false se ainda não há quadro (o chamante usa o
// padrão de teste de fallback — SimpleFrameGenerator).
//
// Protocolo (espelha spikes/s2/pipe-server/src/protocol.rs e
// spikes/s2/shm-server/src/main.rs; se mudar um, mude todos):
//   quadro  : header 28 B little-endian {w:u32,h:u32,format:u32(1=NV12),
//                     seq:u64,pts_us:u64} + NV12 (3 110 400 B)
//   ack     : 16 B LE {seq:u64,read_pts_us:u64} (só no pipe; o servidor
//             bloqueia sem o ack)
//   pts_us  : QPC (QueryPerformanceCounter) em µs — relógio global da
//             máquina, comparável entre processos.
//
#ifndef S2CHANNEL_H
#define S2CHANNEL_H

#include <windows.h>

#include <atomic>
#include <cstdint>
#include <mutex>
#include <thread>
#include <vector>

namespace s2
{
    // Constantes do protocolo (idênticas às do crate s2-pipe-server).
    constexpr uint32_t FrameWidth   = 1920;
    constexpr uint32_t FrameHeight  = 1080;
    constexpr uint32_t FormatNV12   = 1;
    constexpr size_t   HeaderSize   = 28;   // 4+4+4+8+8
    constexpr size_t   Nv12Size     = FrameWidth * FrameHeight * 3 / 2; // 3 110 400
    constexpr size_t   MsgSize      = HeaderSize + Nv12Size;            // 3 110 428
    constexpr size_t   AckSize      = 16;
    constexpr const wchar_t* PipeName = L"\\\\.\\pipe\\LocalCameraSpike";
    constexpr const wchar_t* ShmMapName = L"Global\\LocalCameraSpike";
    constexpr const wchar_t* ShmEventName = L"Global\\LocalCameraSpikeEvt";

    class Channel
    {
    public:
        Channel();
        ~Channel();

        // Inicia o thread de leitura (pipe ou shm, conforme o registro).
        // Deve ser chamado uma única vez.
        void Start();

        // Para o thread e libera recursos. Idempotente.
        void Shutdown();

        // Blita o último quadro recebido (NV12) no buffer do sample.
        // dstPitch: pitch em bytes do sample (pode ser maior que FrameWidth).
        // Devolve false se ainda não há quadro (chamante usa o fallback).
        bool TryFillFrame(BYTE* dst, LONG dstPitch, DWORD bufferLength) const;

        // Estatísticas (para OutputDebugString / medição futura).
        uint64_t FramesRead() const { return m_framesRead.load(std::memory_order_relaxed); }
        uint64_t FramesDropped() const { return m_dropped.load(std::memory_order_relaxed); }
        uint64_t LastLatencyUs() const { return m_lastLatencyUs.load(std::memory_order_relaxed); }

    private:
        Channel(const Channel&) = delete;
        Channel& operator=(const Channel&) = delete;

        void ReaderThread();
        void PipeLoop();
        void ShmLoop();
        bool ShmEnabledFromRegistry() const;
        void SetNoFrame();
        void OnFrameReceived(const uint8_t* msg, uint64_t seq, uint64_t ptsUs, uint64_t readPtsUs);

        // Último quadro (NV12 empacotado, pitch == FrameWidth).
        // Acesso SOMENTE sob m_frameLock.
        mutable std::mutex m_frameLock;
        std::vector<uint8_t> m_latest; // MsgSize
        bool m_hasFrame = false;

        std::atomic<bool> m_stop{false};
        std::thread m_thread;

        uint64_t m_lastSeq = 0; // acessado só no thread de leitura
        std::atomic<uint64_t> m_framesRead{0};
        std::atomic<uint64_t> m_dropped{0};
        std::atomic<uint64_t> m_lastLatencyUs{0};
    };
}

#endif
