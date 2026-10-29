# mock-sender

Finge ser o celular LocalCamera: pareia pela URI, reconecta com a chave
salva, envia vídeo H.264 (Annex-B) com *pacing* e áudio (seno 440 Hz).

## Flags

| Flag | Padrão | Descrição |
|---|---|---|
| `--uri` | — | URI de pareamento (`localcamera://connect?...`); conflita com `--host`/`--token` |
| `--host` | — | Host do PC (com `--token` ou um estado já pareado) |
| `--port` | `48650` | Porta do servidor (ignorada quando a URI traz a sua) |
| `--token` | — | Token de pareamento (32 hex) |
| `--device-id` | do estado ou UUID | ID do dispositivo enviado no `HELLO` |
| `--name` | `Mock Phone` | Nome exibido do celular |
| `--state-file` | — | Arquivo `{deviceId, deviceKey}`; salvo ao parear e reutilizado |
| `--device-key` | — | Chave já pareada (dispensa `--token`) |
| `--file` | — | Arquivo `.h264` (Annex-B) |
| `--loop` | desligado | Reinicia o vídeo ao acabar o arquivo |
| `--fps` | `30` | Quadros por segundo do envio |
| `--audio` | desligado | Abre o socket de áudio e envia o seno |
| `--tone` | `440` | Frequência do seno (Hz) |
| `--seconds` | — | Duração; sem valor, até o arquivo acabar ou Ctrl+C |

## Exemplos

```powershell
# Parear com a URI do QR (grava o estado para a próxima vez)
cargo run -q -p mock-sender -- --uri "localcamera://connect?v=1&hosts=192.168.0.10&port=48650&token=0123456789abcdef0123456789abcdef&pc=pc-1&name=Mock" --state-file mock-state.json

# Reconectar com a chave salva
cargo run -q -p mock-sender -- --host 127.0.0.1 --state-file mock-state.json

# Só áudio por 10 s
cargo run -q -p mock-sender -- --host 127.0.0.1 --token 0123456789abcdef0123456789abcdef --audio --seconds 10

# Vídeo em loop por 30 s
cargo run -q -p mock-sender -- --host 127.0.0.1 --token 0123456789abcdef0123456789abcdef --file fixtures/hd30.h264 --loop --seconds 30
```

O mock termina com `0` (ok), `2` (recusado pelo servidor) ou `1` (erro).
