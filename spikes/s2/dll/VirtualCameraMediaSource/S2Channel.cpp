//
// Spike S2 — implementação do canal (pipe/shm). Ver S2Channel.h e
// spikes/s2/pipe-server/src/protocol.rs para o protocolo.
//
#include "pch.h"

#include <cstring>

namespace s2
{
    // Tempo atual em µs via QPC — o mesmo relógio global da máquina usado
    // pelo servidor (qpc_us em protocol.rs); comparável entre processos.
    static uint64_t NowUs()
    {
        static LARGE_INTEGER freq = [] {
            LARGE_INTEGER f;
            (void)QueryPerformanceFrequency(&f);
            return f;
        }();
        LARGE_INTEGER cnt = {};
        (void)QueryPerformanceCounter(&cnt);
        return (uint64_t)(((uint64_t)cnt.QuadPart * 1000000ULL) / (uint64_t)freq.QuadPart);
    }

    // Lê os campos do header (little-endian; x64 é LE, então memcpy basta).
    static bool ReadHeader(const uint8_t* p, uint32_t& w, uint32_t& h, uint32_t& format, uint64_t& seq, uint64_t& ptsUs)
    {
        memcpy(&w, &p[0], 4);
        memcpy(&h, &p[4], 4);
        memcpy(&format, &p[8], 4);
        memcpy(&seq, &p[12], 8);
        memcpy(&ptsUs, &p[20], 8);
        return (w == FrameWidth) && (h == FrameHeight) && (format == FormatNV12);
    }

    Channel::Channel()
        : m_latest(MsgSize)
    {
    }

    Channel::~Channel()
    {
        Shutdown();
    }

    void Channel::Start()
    {
        if (m_thread.joinable())
        {
            return; // já rodando
        }
        m_stop = false;
        m_thread = std::thread([this] { ReaderThread(); });
    }

    void Channel::Shutdown()
    {
        m_stop = true;
        if (m_thread.joinable())
        {
            m_thread.join();
        }
    }

    void Channel::ReaderThread()
    {
        if (ShmEnabledFromRegistry())
        {
            DEBUG_MSG(L"S2: canal = shm (HKLM\\SOFTWARE\\LocalCameraSpike\\Channel)");
            ShmLoop();
        }
        else
        {
            DEBUG_MSG(L"S2: canal = pipe (padrão)");
            PipeLoop();
        }
    }

    bool Channel::ShmEnabledFromRegistry() const
    {
        HKEY hkey = nullptr;
        if (::RegOpenKeyExW(HKEY_LOCAL_MACHINE, L"SOFTWARE\\LocalCameraSpike", 0,
                            KEY_READ | KEY_WOW64_64KEY, &hkey) != ERROR_SUCCESS)
        {
            return false; // sem a chave -> pipe (padrão)
        }
        wchar_t value[32] = {};
        DWORD size = sizeof(value);
        DWORD type = 0;
        bool shm = false;
        LSTATUS st = ::RegQueryValueExW(hkey, L"Channel", nullptr, &type,
                                        reinterpret_cast<LPBYTE>(value), &size);
        ::RegCloseKey(hkey);
        if ((st == ERROR_SUCCESS) && (type == REG_SZ))
        {
            shm = (wcscmp(value, L"shm") == 0);
            DEBUG_MSG(L"S2: HKLM Channel = %s", value);
        }
        return shm;
    }

    void Channel::SetNoFrame()
    {
        std::lock_guard<std::mutex> lock(m_frameLock);
        m_hasFrame = false;
    }

    void Channel::OnFrameReceived(const uint8_t* msg, uint64_t seq, uint64_t ptsUs, uint64_t readPtsUs)
    {
        // Contagem de perdas por gap de seq (o servidor não repete quadros).
        if (m_lastSeq != 0 && seq > m_lastSeq + 1)
        {
            m_dropped.fetch_add(seq - m_lastSeq - 1, std::memory_order_relaxed);
        }
        m_lastSeq = seq;
        m_framesRead.fetch_add(1, std::memory_order_relaxed);
        m_lastLatencyUs.store(
            (readPtsUs >= ptsUs) ? (readPtsUs - ptsUs) : 0, std::memory_order_relaxed);

        std::lock_guard<std::mutex> lock(m_frameLock);
        memcpy(m_latest.data(), msg, MsgSize);
        m_hasFrame = true;
    }

    //////////////////////////////////////////////////////////////////////////////
    // Pipe — o servidor (app do usuário) publica mensagens {header + NV12} e
    // bloqueia esperando o ack; portanto SEMPRE escrevemos o ack por quadro,
    // senão o servidor para de enviar.
    //////////////////////////////////////////////////////////////////////////////
    void Channel::PipeLoop()
    {
        while (!m_stop.load(std::memory_order_relaxed))
        {
            HANDLE h = ::CreateFileW(PipeName, GENERIC_READ | GENERIC_WRITE, 0,
                                     nullptr, OPEN_EXISTING, 0, nullptr);
            if (h == INVALID_HANDLE_VALUE)
            {
                SetNoFrame();
                ::Sleep(500);
                continue;
            }
            DEBUG_MSG(L"S2 pipe: conectado");
            std::vector<uint8_t> msg(MsgSize);
            while (!m_stop.load(std::memory_order_relaxed))
            {
                DWORD got = 0;
                if (!::ReadFile(h, msg.data(), (DWORD)MsgSize, &got, nullptr))
                {
                    break; // servidor fechou/saiu
                }

                uint32_t w = 0, h2 = 0, format = 0;
                uint64_t seq = 0, ptsUs = 0;
                if (!ReadHeader(msg.data(), w, h2, format, seq, ptsUs))
                {
                    // Header inesperado (não deveria ocorrer); segue o fluxo.
                    DEBUG_MSG(L"S2 pipe: header inesperado %ux%u fmt=%u (seq=%llu)", w, h2, format, (unsigned long long)seq);
                }

                const uint64_t readPtsUs = NowUs();

                // ack {seq, read_pts_us} — reusa o início do buffer.
                memcpy(&msg[0], &seq, 8);
                memcpy(&msg[8], &readPtsUs, 8);
                DWORD written = 0;
                if (!::WriteFile(h, msg.data(), (DWORD)AckSize, &written, nullptr))
                {
                    break;
                }

                OnFrameReceived(msg.data(), seq, ptsUs, readPtsUs);
            }
            ::CloseHandle(h);
            SetNoFrame();
            DEBUG_MSG(L"S2 pipe: desconectado (frames=%llu, drops=%llu); tentando de novo em 500 ms",
                      (unsigned long long)m_framesRead.load(), (unsigned long long)m_dropped.load());
            if (!m_stop.load(std::memory_order_relaxed))
            {
                ::Sleep(500);
            }
        }
    }

    //////////////////////////////////////////////////////////////////////////////
    // Shm — o servidor escreve {header + NV12} no mapa e dá SetEvent; aqui
    // esperamos o evento e copiamos. Se não chegar evento em 200 ms, o
    // servidor provavelmente saiu -> reabrir.
    //////////////////////////////////////////////////////////////////////////////
    void Channel::ShmLoop()
    {
        while (!m_stop.load(std::memory_order_relaxed))
        {
            HANDLE hmap = ::OpenFileMappingW(FILE_MAP_COPY, FALSE, ShmMapName);
            if (hmap == nullptr)
            {
                SetNoFrame();
                ::Sleep(500);
                continue;
            }
            HANDLE hevt = ::OpenEventW(SYNCHRONIZE, FALSE, ShmEventName);
            if (hevt == nullptr)
            {
                ::CloseHandle(hmap);
                SetNoFrame();
                ::Sleep(500);
                continue;
            }
            LPVOID view = ::MapViewOfFile(hmap, FILE_MAP_COPY, 0, 0, MsgSize);
            if (view == nullptr)
            {
                ::CloseHandle(hevt);
                ::CloseHandle(hmap);
                SetNoFrame();
                ::Sleep(500);
                continue;
            }
            DEBUG_MSG(L"S2 shm: mapa aberto");
            const uint8_t* p = static_cast<const uint8_t*>(view);
            while (!m_stop.load(std::memory_order_relaxed))
            {
                const DWORD wr = ::WaitForSingleObject(hevt, 200);
                if (wr != WAIT_OBJECT_0)
                {
                    break; // sem evento: servidor parado/saiu
                }
                uint32_t w = 0, h2 = 0, format = 0;
                uint64_t seq = 0, ptsUs = 0;
                if (!ReadHeader(p, w, h2, format, seq, ptsUs))
                {
                    DEBUG_MSG(L"S2 shm: header inesperado %ux%u fmt=%u (seq=%llu)", w, h2, format, (unsigned long long)seq);
                    continue;
                }
                const uint64_t readPtsUs = NowUs();
                OnFrameReceived(p, seq, ptsUs, readPtsUs);
            }
            ::UnmapViewOfFile(view);
            ::CloseHandle(hevt);
            ::CloseHandle(hmap);
            SetNoFrame();
            DEBUG_MSG(L"S2 shm: desconectado (frames=%llu, drops=%llu); tentando de novo em 500 ms",
                      (unsigned long long)m_framesRead.load(), (unsigned long long)m_dropped.load());
            if (!m_stop.load(std::memory_order_relaxed))
            {
                ::Sleep(500);
            }
        }
    }

    //////////////////////////////////////////////////////////////////////////////
    // Blit NV12 do buffer privado (pitch == FrameWidth) para o sample
    // (pitch = dstPitch). Mesmo layout do sample S1 (RGB32ToNV12Frame):
    //   Y  : FrameHeight linhas de FrameWidth bytes, passo dstPitch
    //   UV : base = h*dstPitch; FrameHeight/2 linhas de FrameWidth bytes
    //        (U,V intercalados), passo dstPitch
    //////////////////////////////////////////////////////////////////////////////
    bool Channel::TryFillFrame(BYTE* dst, LONG dstPitch, DWORD bufferLength) const
    {
        if ((dst == nullptr) || (dstPitch <= 0) || (dstPitch > (LONG)FrameWidth))
        {
            return false; // dstPitch menor que a largura não faz sentido
        }
        // Sanidade: buffer precisa caber Y + UV com o pitch informado.
        const size_t needed = (size_t)FrameHeight * (size_t)dstPitch * 3 / 2;
        if (bufferLength < needed)
        {
            return false;
        }

        std::lock_guard<std::mutex> lock(m_frameLock);
        if (!m_hasFrame)
        {
            return false;
        }
        const uint8_t* src = m_latest.data() + HeaderSize;
        const size_t sw = FrameWidth;

        // Plano Y.
        for (uint32_t r = 0; r < FrameHeight; ++r)
        {
            memcpy(dst + (size_t)r * (size_t)dstPitch, src + (size_t)r * sw, sw);
        }
        // Plano UV (intercalado, sw bytes por linha).
        BYTE* dstUV = dst + (size_t)FrameHeight * (size_t)dstPitch;
        const uint8_t* srcUV = src + sw * FrameHeight;
        for (uint32_t u = 0; u < FrameHeight / 2; ++u)
        {
            memcpy(dstUV + (size_t)u * (size_t)dstPitch, srcUV + (size_t)u * sw, sw);
        }
        return true;
    }
}
