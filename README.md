# LocalCamera

O LocalCamera transforma um celular Android em webcam e microfone para Windows 11, conectado por Wi-Fi ou USB.

## Pré-requisitos

- Node.js 22 ou superior (`.nvmrc`; `engines` no `package.json`) e pnpm 12.15.0 (`packageManager`).
- Rust 1.99.1 com `rustfmt` e `clippy` (`rust-toolchain.toml`).
- Para Android: JDK 21 (o app declara Java 17, mas Capacitor/plugin compilam com Java 21); Android SDK com `compileSdk`/`targetSdk` 36 e `minSdk` 24 (`mobile/android/variables.gradle`); Gradle Wrapper 8.14.3 (`mobile/android/gradle/wrapper/gradle-wrapper.properties`).
- Windows 11 para o desktop e WebView2 conforme instalação do Windows/ambiente. O repositório não fixa versão de WebView2.

## Desenvolvimento

Na raiz do repositório:

```powershell
pnpm install
pnpm run lint
pnpm run typecheck
pnpm run test
pnpm run ci
pnpm e2e
pnpm --filter desktop tauri dev
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
Push-Location mobile/android
.\gradlew.bat testDebugUnitTest lintDebug
Pop-Location
```

Os scripts pnpm estão definidos no `package.json`; `pnpm run ci` executa `scripts/ci.ps1`. Os comandos Rust requerem o toolchain declarado em `rust-toolchain.toml`; os testes Android usam o Gradle Wrapper em `mobile/android`.

### E2E (Playwright)

O Playwright inicia servidores Vite próprios com `--strictPort` e não reutiliza servidores existentes. Os padrões são `1421` (desktop) e `5199` (mobile); configure portas TCP livres se esses padrões estiverem ocupados:

```powershell
$env:LC_E2E_DESKTOP_PORT = "1421"
$env:LC_E2E_MOBILE_PORT = "5199"
pnpm e2e
Remove-Item Env:LC_E2E_DESKTOP_PORT, Env:LC_E2E_MOBILE_PORT
```

## Estrutura

- `crates/` — crates Rust do protocolo e servidor.
- `desktop/` — aplicativo desktop Tauri.
- `mobile/` — aplicativo Android.
- `packages/` — pacotes compartilhados.
- `tools/` — ferramentas auxiliares.
- `e2e/` — testes ponta a ponta.
- `scripts/` — automação e validações.
- `fixtures/` — dados sintéticos de teste.
- `spikes/` — experimentos técnicos.
- `docs/` — documentação, planos e evidências.

## Documentação

- [Resumo técnico](docs/CHEATSHEET.md)
- [Estratégia de testes](docs/testing.md)
- [Tarefas](docs/tasks/README.md)
- [Progresso](PROGRESS.md)
